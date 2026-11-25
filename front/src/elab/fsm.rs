use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll, Waker},
};

use crate::{
    air,
    domain::Domain,
    elab,
    module::{Action, AsyncCx},
    value::{IntoValues, Shape, U, Value},
};

use super::{RegisterHandle, with_context};

/// The async FSM's state register; unlike the public [`crate::reg::Reg`],
/// it is not branded with a domain marker, because it always belongs to
/// the enclosing module's clock.
/// Private domain marker for the async FSM's state register. The state
/// register is internal to the lowering and never escapes to user code;
/// its clock is the enclosing module's, resolved at elaboration time.
#[derive(Debug, Clone, Copy)]
struct Internal;

impl Domain for Internal {}

/// The async FSM's state register. Deliberately branded with the
/// private [`Internal`] domain: it never escapes to user code, and its
/// clock is the enclosing module's, resolved at elaboration time.
#[derive(Debug, Clone, Copy)]
struct StateReg {
    id: RegisterHandle,
}

impl StateReg {
    fn new() -> Self {
        let (id, _) = super::register_default::<Internal, _>(Some(U::<32>::zero()));
        Self { id }
    }

    fn uninit<D: Domain, T: Shape>() -> RegisterHandle {
        super::register_default::<D, T>(None).0
    }

    fn read(&self) -> Value<Internal, U<32>> {
        elab::read_register::<Internal, U<32>>(self.id)
    }

    fn write(&self, value: Value<Internal, U<32>>) {
        elab::write_register::<Internal, U<32>>(self.id, value);
    }
}

pub struct Flow {
    state: StateReg,
    next: usize,
    current: usize,
    suppress: bool,
    done: Option<usize>,
}

fn state_value(value: usize) -> Value<Internal, U<32>> {
    super::uint_from_u128::<Internal, 32>(value as u128)
}

#[must_use]
pub fn unit_value() -> super::ExprHandle {
    with_context(|cx| {
        let unit = cx.unit();
        cx.push_expr(unit)
    })
}

fn with_flow<R>(f: impl FnOnce(&mut Flow) -> R) -> R {
    with_context(|cx| {
        f(cx.async_flow
            .as_mut()
            .expect("no async method lowering in progress"))
    })
}

const fn transition(flow: &mut Flow) -> StateReg {
    let next = flow.next;
    flow.current = next;
    flow.next += 1;
    flow.state
}

fn poll_segment<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut task = TaskContext::from_waker(Waker::noop());
    let poll = future.poll(&mut task);
    if matches!(poll, Poll::Pending) {
        let (suppress, state, next) = with_flow(|flow| {
            let suppress = flow.suppress;
            flow.suppress = false;
            if !suppress {
                let next = flow.next;
                flow.current = next;
                flow.next += 1;
            }
            (suppress, flow.state, flow.next - 1)
        });
        if !suppress {
            state.write(state_value(next));
        }
    }
    poll
}

fn begin_flow(state: StateReg) {
    with_context(|cx| {
        assert!(cx.frames.is_empty(), "define async bodies at module scope");
        assert!(cx.async_flow.is_none(), "nested async body definition");
        cx.async_flow = Some(Flow {
            state,
            next: 1,
            current: 0,
            suppress: false,
            done: None,
        });
    });
}

fn close_state(state: StateReg, index: usize) {
    let guard = elab::compare(air::BinaryOp::Equal, state.read(), state_value(index));
    with_context(|cx| {
        let guard = cx.typed_expr::<bool>(guard.handle());
        let block = cx.end_body();
        let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
        cx.add_rule(air::Rule {
            name,
            guard: Some(guard),
            body: block,
        });
    });
}

fn drive<F: Future + ?Sized>(future: &mut Pin<Box<F>>, mut finish: impl FnMut(F::Output)) {
    loop {
        let current = with_flow(|flow| flow.current);
        // Passing `Context::begin_body` directly fails to infer here.
        #[allow(clippy::redundant_closure_for_method_calls)]
        with_context(|cx| cx.begin_body());
        let mut suspended = true;
        match poll_segment(future.as_mut()) {
            Poll::Pending => {}
            Poll::Ready(output) => {
                suspended = false;
                finish(output);
            }
        }
        close_state(with_flow(|flow| flow.state), current);
        if !suspended {
            break;
        }
    }
}

fn finish_state(value: usize) {
    let state = with_flow(|flow| {
        flow.done = Some(value);
        flow.current = value;
        flow.state
    });
    state.write(state_value(value));
}

pub fn define_async_method<'a, D: Domain, In, Out, F>(
    f: F,
) -> (Action<D, In, ()>, Action<D, (), Out>)
where
    In: Shape,
    Out: Shape,
    F: AsyncFnOnce(AsyncCx, Value<D, In>) -> Out::Values<D> + 'a,
{
    let state = StateReg::new();
    let (in_unit, out_unit) = with_context(|cx| {
        (
            matches!(cx.ty::<In>(), air::Type::Unit),
            matches!(cx.ty::<Out>(), air::Type::Unit),
        )
    });
    let args = (!in_unit).then(|| StateReg::uninit::<D, In>());
    let result = (!out_unit).then(|| StateReg::uninit::<D, Out>());
    begin_flow(state);

    let mut slot: Option<Pin<Box<dyn Future<Output = Out::Values<D>> + 'a>>> = None;
    let mut body = Some(f);

    let req = Action::from_handle(super::define_action::<D, In, ()>(
        Some(elab::compare(air::BinaryOp::Equal, state.read(), state_value(0)).handle()),
        |_cx, args_in| {
            if let Some(id) = args {
                super::write_register::<D, In>(id, args_in);
                state.write(state_value(1));
                with_flow(|flow| flow.current = 1);
            } else {
                let f = body.take().expect("async method body was already consumed");
                let mut future = Box::pin(f(AsyncCx::new(), Value::from_handle(unit_value())));
                match poll_segment(future.as_mut()) {
                    Poll::Pending => {}
                    Poll::Ready(value) => {
                        if let Some(id) = result {
                            super::write_register::<D, Out>(id, Out::pack_values(value));
                        }
                        let done = with_flow(|flow| {
                            let done = flow.next;
                            flow.next += 1;
                            done
                        });
                        finish_state(done);
                    }
                }
                slot = Some(future);
            }
        },
    ));

    let mut future = match slot.take() {
        Some(future) => future,
        None => Box::pin(
            body.take().expect("async method body was already consumed")(
                AsyncCx::new(),
                elab::read_register::<D, In>(args.expect("args register missing")),
            ),
        ),
    };
    drive(&mut future, |value| {
        if let Some(id) = result {
            super::write_register::<D, Out>(id, Out::pack_values(value));
        }
        let done = with_flow(|flow| {
            let done = flow.next;
            flow.next += 1;
            done
        });
        finish_state(done);
    });

    let done = with_flow(|flow| flow.done.expect("async method body did not complete"));
    let out = Action::from_handle(super::define_action::<D, (), Out>(
        Some(elab::compare(air::BinaryOp::Equal, state.read(), state_value(done)).handle()),
        |_cx, _packed| {
            state.write(state_value(0));
            match result {
                Some(id) => Out::make_values(elab::read_register::<D, Out>(id)),
                None => Out::make_values(Value::from_handle(unit_value())),
            }
        },
    ));

    with_context(|cx| cx.async_flow = None);
    (req, out)
}

pub fn define_async_rule<'a, F, R>(f: F)
where
    F: AsyncFnOnce(AsyncCx) -> R + 'a,
{
    let state = StateReg::new();
    begin_flow(state);

    let mut slot: Option<Pin<Box<dyn Future<Output = R> + 'a>>> = None;

    super::define_rule(
        Some(elab::compare(air::BinaryOp::Equal, state.read(), state_value(0)).handle()),
        |_cx| {
            let mut future = Box::pin(f(AsyncCx::new()));
            match poll_segment(future.as_mut()) {
                Poll::Pending => {}
                Poll::Ready(_) => {
                    let done = with_flow(|flow| {
                        let done = flow.next;
                        flow.next += 1;
                        done
                    });
                    finish_state(done);
                }
            }
            slot = Some(future);
        },
    );

    let mut future = slot.take().expect("async rule body was not polled");
    drive(&mut future, |_| {
        let done = with_flow(|flow| {
            let done = flow.next;
            flow.next += 1;
            done
        });
        finish_state(done);
    });

    with_context(|cx| cx.async_flow = None);
}

#[must_use]
pub fn lower_async_if<'a, D: Domain, R: IntoValues<D>>(
    cond: Value<D, bool>,
    then: Pin<Box<dyn Future<Output = R> + 'a>>,
    else_: Pin<Box<dyn Future<Output = R> + 'a>>,
) -> Option<super::RegisterHandle> {
    let out_unit = with_context(|cx| matches!(cx.ty::<R::Out>(), air::Type::Unit));
    let result = (!out_unit).then(|| super::register_default::<D, R::Out>(None).0);

    let (then_start, else_start, merge) = with_flow(|flow| {
        let then_start = flow.next;
        let else_start = then_start + 1;
        let merge = else_start + 1;
        flow.next = merge + 1;
        (then_start, else_start, merge)
    });

    let state = with_flow(|flow| flow.state);
    let then_state = state_value(then_start);
    let else_state = state_value(else_start);
    with_context(|cx| {
        let register = cx.register_for_write(state.id.index);
        let condition = cx.typed_expr::<bool>(cond.handle());
        let then_value = cx.typed_expr::<U<32>>(then_state.handle());
        let else_value = cx.typed_expr::<U<32>>(else_state.handle());
        cx.emit(air::Statement::If {
            condition,
            then_body: air::Block {
                statements: cx.alloc_slice([air::Statement::RegisterWrite {
                    register,
                    value: then_value,
                }]),
            },
            else_body: Some(air::Block {
                statements: cx.alloc_slice([air::Statement::RegisterWrite {
                    register,
                    value: else_value,
                }]),
            }),
        });
    });

    drive_chain(then, then_start, merge, result);
    drive_chain(else_, else_start, merge, result);

    with_flow(|flow| {
        flow.current = merge;
        flow.suppress = true;
    });
    result
}

fn drive_chain<'a, D: Domain, R: IntoValues<D>>(
    mut future: Pin<Box<dyn Future<Output = R> + 'a>>,
    start: usize,
    merge: usize,
    result: Option<super::RegisterHandle>,
) {
    let state = with_flow(|flow| flow.state);
    let mut current = start;
    loop {
        // Passing `Context::begin_body` directly fails to infer here.
        #[allow(clippy::redundant_closure_for_method_calls)]
        with_context(|cx| cx.begin_body());
        let ready = match poll_segment(future.as_mut()) {
            Poll::Pending => None,
            Poll::Ready(value) => Some(value),
        };
        if let Some(value) = ready {
            if let Some(handle) = result {
                super::write_register(handle, R::Out::pack_values::<D>(value.into_values()));
            }
            state.write(state_value(merge));
            with_flow(|flow| flow.current = merge);
            close_state(state, current);
            return;
        }
        close_state(state, current);
        current = with_flow(|flow| flow.current);
    }
}
