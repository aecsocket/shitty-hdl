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
    ops::BitCmp,
    value::{IntoValues, Shape, U, Value},
};

use super::{Context, RegisterHandle, with_context};

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
    /// Created lazily: bodies that never suspend never need one.
    state: Option<StateReg>,
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
            .expect("suspend is only supported inside async rule and method bodies"))
    })
}

fn flow_state() -> StateReg {
    let existing = with_flow(|flow| flow.state);
    existing.unwrap_or_else(|| {
        let state = StateReg::new();
        with_flow(|flow| *flow.state.insert(state))
    })
}

fn poll_segment<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut task = TaskContext::from_waker(Waker::noop());
    let poll = future.poll(&mut task);
    if matches!(poll, Poll::Pending) {
        let next = with_flow(|flow| {
            let suppress = flow.suppress;
            flow.suppress = false;
            if suppress {
                None
            } else {
                let next = flow.next;
                flow.current = next;
                flow.next += 1;
                Some(next)
            }
        });
        if let Some(next) = next {
            flow_state().write(state_value(next));
        }
    }
    poll
}

fn begin_flow(state: Option<StateReg>) {
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
        close_state(flow_state(), current);
        if !suspended {
            break;
        }
    }
}

fn finish_state(value: usize) {
    with_flow(|flow| {
        flow.done = Some(value);
        flow.current = value;
    });
    flow_state().write(state_value(value));
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
    begin_flow(Some(state));

    let mut slot: Option<Pin<Box<dyn Future<Output = Out::Values<D>> + 'a>>> = None;
    let mut body = Some(f);

    let req = Action::from_handle(super::define_action_sync::<D, In, ()>(
        Some(elab::compare(air::BinaryOp::Equal, state.read(), state_value(0)).handle()),
        |_cx, args_in| {
            if let Some(id) = args {
                super::write_register::<D, In>(id, args_in);
                state.write(state_value(1));
                // The body's first segment runs in state 1 (one cycle after
                // the request); state 0 is consumed, so state 1 must be
                // consumed as well - otherwise the first `if` would aliase
                // its branches onto state 1.
                with_flow(|flow| {
                    flow.current = 1;
                    flow.next = 2;
                });
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
    let out = Action::from_handle(super::define_action_sync::<D, (), Out>(
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

/// Lowers an async rule body.
///
/// A body that completes on its first poll never crosses a clock edge, so
/// it is emitted as a plain rule that fires whenever its guard holds; only
/// a suspending body needs the state machine.
pub fn define_async_rule<'a, F, R>(guard: Option<super::ExprHandle>, f: F)
where
    F: AsyncFnOnce(AsyncCx) -> R + 'a,
    R: 'a,
{
    with_context(|cx| {
        assert!(
            cx.frames.is_empty(),
            "define rules at module scope, not inside action bodies"
        );
    });
    begin_flow(None);

    #[allow(
        clippy::redundant_closure_for_method_calls,
        reason = "passing `Context::begin_body` directly fails to infer"
    )]
    with_context(|cx| cx.begin_body());
    let mut future = Box::pin(f(AsyncCx::new()));
    let poll = poll_segment(future.as_mut());

    match poll {
        Poll::Ready(_) => {
            with_context(|cx| {
                let block = cx.end_body();
                let guard = guard.map(|handle| cx.typed_expr::<bool>(handle));
                let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
                cx.add_rule(air::Rule {
                    name,
                    guard,
                    body: block,
                });
            });
        }
        Poll::Pending => {
            let state = flow_state();
            with_context(|cx| {
                // The guard is built and resolved while the speculative
                // body is still open: its exprs belong to this scope.
                let check = elab_compare_inline(cx, state, 0);
                let guard = match guard {
                    Some(handle) => {
                        let cond = cx.typed_expr::<bool>(handle);
                        let expr = cx.alloc(air::Expr {
                            ty: check.ty,
                            kind: air::ExprKind::Binary {
                                op: air::BinaryOp::And,
                                lhs: cond,
                                rhs: check,
                            },
                        });
                        cx.push_expr(expr);
                        cx.expr(handle)
                    }
                    None => check,
                };
                let block = cx.end_body();
                let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
                cx.add_rule(air::Rule {
                    name,
                    guard: Some(guard),
                    body: block,
                });
            });
            drive(&mut future, |_| {
                let done = with_flow(|flow| {
                    let done = flow.next;
                    flow.next += 1;
                    done
                });
                finish_state(done);
            });
        }
    }

    with_context(|cx| cx.async_flow = None);
}

/// The result of lowering an async `if`: branches that both completed
/// speculatively collapse into a combinational mux; otherwise the `if`
/// joins the enclosing state machine.
pub enum LoweredIf<D: Domain, R: IntoValues<D>> {
    Mux(Value<D, R::Out>),
    Fsm(Option<RegisterHandle>),
}

#[must_use]
pub fn lower_async_if<'a, D: Domain, R: IntoValues<D>>(
    cond: Value<D, bool>,
    mut then: Pin<Box<dyn Future<Output = R> + 'a>>,
    mut else_: Pin<Box<dyn Future<Output = R> + 'a>>,
) -> LoweredIf<D, R> {
    let cond_branch = super::capture_branch(|| cond);

    #[allow(
        clippy::redundant_closure_for_method_calls,
        reason = "passing `Context::begin_body` directly fails to infer"
    )]
    with_context(|cx| cx.begin_body());
    let then_poll = poll_segment(then.as_mut());
    // Pack while the branch's own body is the innermost frame, so the
    // result exprs are created in (and can be resolved from) its scope.
    let then_pending = matches!(then_poll, Poll::Pending);
    let mut then_packed = match then_poll {
        Poll::Ready(value) => Some(R::Out::pack_values::<D>(value.into_values())),
        Poll::Pending => None,
    };
    let then_next = then_pending.then(|| with_flow(|flow| flow.current));
    #[allow(
        clippy::redundant_closure_for_method_calls,
        reason = "passing `Context::begin_body` directly fails to infer"
    )]
    with_context(|cx| cx.begin_body());
    let else_poll = poll_segment(else_.as_mut());
    let else_pending = matches!(else_poll, Poll::Pending);
    let mut else_packed = match else_poll {
        Poll::Ready(value) => Some(R::Out::pack_values::<D>(value.into_values())),
        Poll::Pending => None,
    };
    let else_next = else_pending.then(|| with_flow(|flow| flow.current));

    let then_taken = then_packed.take();
    let else_taken = else_packed.take();
    match (then_taken, else_taken) {
        (Some(then_packed), Some(else_packed)) => {
            let (then_branch, else_branch) = with_context(|cx| {
                // Resolve each branch's result while its own body is the
                // innermost open frame; the scope check would reject the
                // exprs once the bodies are closed.
                let else_result = cx.expr(else_packed.handle());
                let else_block = cx.end_body();
                let then_result = cx.expr(then_packed.handle());
                let then_block = cx.end_body();
                let else_branch = push_branch_cx(cx, else_block, else_result);
                let then_branch = push_branch_cx(cx, then_block, then_result);
                (then_branch, else_branch)
            });
            let value =
                super::finish_if::<D, R::Out>(vec![(cond_branch, then_branch)], Some(else_branch));
            LoweredIf::Mux(value)
        }
        (then_packed, else_packed) => {
            let then_pending = then_packed.is_none();
            let else_pending = else_packed.is_none();
            // Only a suspending branch needs the state machine, so the states
            // are allocated (and the state register created) only now, after
            // the speculative polls.
            let (then_start, else_start, merge) = with_flow(|flow| {
                let then_start = flow.next;
                let else_start = then_start + 1;
                let merge = else_start + 1;
                flow.next = merge + 1;
                (then_start, else_start, merge)
            });
            let state = flow_state();
            let out_unit = with_context(|cx| matches!(cx.ty::<R::Out>(), air::Type::Unit));
            let result = (!out_unit).then(|| super::register_default::<D, R::Out>(None).0);
            with_context(|cx| {
                // Same LIFO resolution as the mux path: each branch's
                // result expr is only in scope while its body is open.
                let else_result = else_packed.map(|value| cx.expr(value.handle()));
                let else_block = cx.end_body();
                let then_result = then_packed.map(|value| cx.expr(value.handle()));
                let then_block = cx.end_body();
                let then_block = finish_entry_cx(cx, state, then_block, then_result, merge, result);
                let else_block = finish_entry_cx(cx, state, else_block, else_result, merge, result);
                let then_guard = elab_compare_inline(cx, state, then_start);
                let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
                cx.add_rule(air::Rule {
                    name,
                    guard: Some(then_guard),
                    body: then_block,
                });
                let else_guard = elab_compare_inline(cx, state, else_start);
                let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
                cx.add_rule(air::Rule {
                    name,
                    guard: Some(else_guard),
                    body: else_block,
                });
            });

            // The branch selection happens in the cycle the enclosing
            // segment runs, so it is emitted there - after the branch
            // bodies above have been closed, back at the enclosing scope.
            with_context(|cx| {
                let state_register = cx.register_for_write(state.id.index);
                let condition = cx.typed_expr::<bool>(cond.handle());
                let then_value = state_value_cx(cx, then_start);
                let else_value = state_value_cx(cx, else_start);
                cx.emit(air::Statement::If {
                    condition,
                    then_body: air::Block {
                        statements: cx.alloc_slice([air::Statement::RegisterWrite {
                            register: state_register,
                            value: then_value,
                        }]),
                    },
                    else_body: Some(air::Block {
                        statements: cx.alloc_slice([air::Statement::RegisterWrite {
                            register: state_register,
                            value: else_value,
                        }]),
                    }),
                });
            });

            if then_pending {
                drive_branch_rest(then, then_next.expect("pending branch"), merge, result);
            }
            if else_pending {
                drive_branch_rest(else_, else_next.expect("pending branch"), merge, result);
            }

            with_flow(|flow| {
                flow.current = merge;
                flow.suppress = true;
            });
            LoweredIf::Fsm(result)
        }
    }
}

/// The states allocated for a lowered loop. The enclosing segment enters
/// `head`, which selects between `body` and `exit`; the body loops back to
/// `head`, and the enclosing continuation is elaborated in `exit`.
struct LoopStates {
    head: usize,
    body: usize,
    exit: usize,
}

/// Allocates the loop's head/body/exit states and emits the head rule (which
/// evaluates the condition and branches) plus the enclosing state's transition
/// into the head. The caller is responsible for driving the body and for
/// leaving the enclosing continuation in `exit`.
fn begin_loop<D: Domain>(cond: Value<D, bool>) -> LoopStates {
    let cond_branch = super::capture_branch(|| cond);
    let (head, body, exit) = with_flow(|flow| {
        let head = flow.next;
        let body = head + 1;
        let exit = body + 1;
        flow.next = exit + 1;
        (head, body, exit)
    });
    let state = flow_state();
    with_context(|cx| {
        let (statements, condition) = {
            let branch = &cx.branches[cond_branch.index];
            (branch.block.statements, branch.result)
        };
        let mut statements = statements.to_vec();
        let state_register = cx.register_for_write(state.id.index);
        let body_value = state_value_cx(cx, body);
        let exit_value = state_value_cx(cx, exit);
        statements.push(air::Statement::If {
            condition,
            then_body: air::Block {
                statements: cx.alloc_slice([air::Statement::RegisterWrite {
                    register: state_register,
                    value: body_value,
                }]),
            },
            else_body: Some(air::Block {
                statements: cx.alloc_slice([air::Statement::RegisterWrite {
                    register: state_register,
                    value: exit_value,
                }]),
            }),
        });
        let block = air::Block {
            statements: cx.alloc_slice(statements),
        };
        let guard = elab_compare_inline(cx, state, head);
        let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
        cx.add_rule(air::Rule {
            name,
            guard: Some(guard),
            body: block,
        });
        let state_register = cx.register_for_write(state.id.index);
        let head_value = state_value_cx(cx, head);
        cx.emit(air::Statement::RegisterWrite {
            register: state_register,
            value: head_value,
        });
    });
    LoopStates { head, body, exit }
}

/// Drives the body of a loop until it completes, adding one rule per segment.
/// A completed segment transitions to `target`; a suspended one transitions
/// to the next segment allocated by [`poll_segment`].
fn drive_loop_body<'a>(
    mut future: Pin<Box<dyn Future<Output = ()> + 'a>>,
    first: usize,
    target: usize,
) {
    let state = flow_state();
    with_flow(|flow| flow.current = first);
    loop {
        let current = with_flow(|flow| flow.current);
        #[allow(
            clippy::redundant_closure_for_method_calls,
            reason = "passing `Context::begin_body` directly fails to infer"
        )]
        with_context(|cx| cx.begin_body());
        let poll = poll_segment(future.as_mut());
        let ready = matches!(poll, Poll::Ready(()));
        with_context(|cx| {
            let block = cx.end_body();
            let mut statements = block.statements.to_vec();
            if ready {
                let state_register = cx.register_for_write(state.id.index);
                let target_value = state_value_cx(cx, target);
                statements.push(air::Statement::RegisterWrite {
                    register: state_register,
                    value: target_value,
                });
            }
            let body = air::Block {
                statements: cx.alloc_slice(statements),
            };
            let guard = elab_compare_inline(cx, state, current);
            let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
            cx.add_rule(air::Rule {
                name,
                guard: Some(guard),
                body,
            });
        });
        if ready {
            with_flow(|flow| flow.current = target);
            return;
        }
        // `poll_segment` advanced `flow.current` to the next segment.
    }
}

/// Lowers `while cond { body }`: the head evaluates the condition each cycle,
/// the body loops back to it, and the enclosing async body resumes in the
/// exit state once the condition is false.
pub fn lower_async_while<'a, D: Domain>(
    cond: Value<D, bool>,
    body: Pin<Box<dyn Future<Output = ()> + 'a>>,
) {
    let states = begin_loop(cond);
    drive_loop_body(body, states.body, states.head);
    with_flow(|flow| {
        flow.current = states.exit;
        flow.suppress = true;
    });
}

/// Lowers `for i in start..end { body }` (or `..=`) over a hardware range.
///
/// The range is captured on entry: `start` and `end` are written into
/// registers, the index register drives the head comparison, and an extra
/// step state increments the index (or, for an inclusive range, exits when
/// the index reaches the bound, avoiding overflow).
#[allow(
    clippy::too_many_arguments,
    reason = "the loop's index, start, bound, and end state are elaborated together"
)]
pub fn lower_async_for<'a, D, T>(
    body: Pin<Box<dyn Future<Output = ()> + 'a>>,
    index: RegisterHandle,
    index_value: Value<D, T>,
    start: Value<D, T>,
    bound: RegisterHandle,
    bound_value: Value<D, T>,
    end: Value<D, T>,
    inclusive: bool,
) where
    D: Domain,
    T: Shape + BitCmp,
{
    let cond = if inclusive {
        T::le(index_value, bound_value)
    } else {
        T::lt(index_value, bound_value)
    };
    let increment = T::add(index_value, T::from_u128(1));
    let step_condition = inclusive.then(|| T::eq(index_value, bound_value));

    let states = begin_loop(cond);
    let step = with_flow(|flow| {
        let step = flow.next;
        flow.next += 1;
        step
    });
    let state = flow_state();

    with_context(|cx| {
        let index_register = cx.register_for_write(index.index);
        let start = cx.typed_expr::<T>(start.handle());
        cx.emit(air::Statement::RegisterWrite {
            register: index_register,
            value: start,
        });
        let bound_register = cx.register_for_write(bound.index);
        let end = cx.typed_expr::<T>(end.handle());
        cx.emit(air::Statement::RegisterWrite {
            register: bound_register,
            value: end,
        });
    });

    with_context(|cx| {
        let state_register = cx.register_for_write(state.id.index);
        let index_register = cx.register_for_write(index.index);
        let increment = cx.typed_expr::<T>(increment.handle());
        let head_value = state_value_cx(cx, states.head);
        let exit_value = state_value_cx(cx, states.exit);
        let step_body = match step_condition {
            Some(condition) => {
                let condition = cx.typed_expr::<bool>(condition.handle());
                air::Block {
                    statements: cx.alloc_slice([air::Statement::If {
                        condition,
                        then_body: air::Block {
                            statements: cx.alloc_slice([air::Statement::RegisterWrite {
                                register: state_register,
                                value: exit_value,
                            }]),
                        },
                        else_body: Some(air::Block {
                            statements: cx.alloc_slice([
                                air::Statement::RegisterWrite {
                                    register: index_register,
                                    value: increment,
                                },
                                air::Statement::RegisterWrite {
                                    register: state_register,
                                    value: head_value,
                                },
                            ]),
                        }),
                    }]),
                }
            }
            None => air::Block {
                statements: cx.alloc_slice([
                    air::Statement::RegisterWrite {
                        register: index_register,
                        value: increment,
                    },
                    air::Statement::RegisterWrite {
                        register: state_register,
                        value: head_value,
                    },
                ]),
            },
        };
        let guard = elab_compare_inline(cx, state, step);
        let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
        cx.add_rule(air::Rule {
            name,
            guard: Some(guard),
            body: step_body,
        });
    });

    drive_loop_body(body, states.body, step);
    with_flow(|flow| {
        flow.current = states.exit;
        flow.suppress = true;
    });
}

/// The [`state_value`] literal, built on an already-borrowed context.
fn state_value_cx<'dsn>(cx: &mut Context<'dsn>, value: usize) -> &'dsn air::Expr<'dsn> {
    let words = [value as u64];
    let lit = cx.bits::<U<32>>(&words);
    cx.push_expr(lit);
    lit
}

/// Closes a branch's entry segment: a completed branch gets its result
/// write and the transition to the merge state appended; a suspending
/// branch's block already carries its transition.
fn finish_entry_cx<'dsn>(
    cx: &mut Context<'dsn>,
    state: StateReg,
    block: air::Block<'dsn>,
    packed: Option<&'dsn air::Expr<'dsn>>,
    merge: usize,
    result: Option<RegisterHandle>,
) -> air::Block<'dsn> {
    match packed {
        None => block,
        Some(packed) => {
            let mut statements = block.statements.to_vec();
            if let Some(register) = result {
                let register = cx.register_for_write(register.index);
                statements.push(air::Statement::RegisterWrite {
                    register,
                    value: packed,
                });
            }
            let state_register = cx.register_for_write(state.id.index);
            let merge_value = state_value_cx(cx, merge);
            statements.push(air::Statement::RegisterWrite {
                register: state_register,
                value: merge_value,
            });
            air::Block {
                statements: cx.alloc_slice(statements),
            }
        }
    }
}

/// Drives the remaining segments of a suspending branch until merge.
fn drive_branch_rest<'a, D: Domain, R: IntoValues<D>>(
    mut future: Pin<Box<dyn Future<Output = R> + 'a>>,
    first_current: usize,
    merge: usize,
    result: Option<RegisterHandle>,
) {
    let state = flow_state();
    let mut current = first_current;
    loop {
        #[allow(
            clippy::redundant_closure_for_method_calls,
            reason = "passing `Context::begin_body` directly fails to infer"
        )]
        with_context(|cx| cx.begin_body());
        let poll = poll_segment(future.as_mut());
        let packed = match poll {
            Poll::Ready(value) => Some(R::Out::pack_values::<D>(value.into_values())),
            Poll::Pending => None,
        };
        let mut ready = false;
        with_context(|cx| {
            // Resolve the result while the segment's body is still open.
            let packed = packed.map(|value| cx.expr(value.handle()));
            let block = cx.end_body();
            let block = match packed {
                None => block,
                Some(packed) => {
                    ready = true;
                    let mut statements = block.statements.to_vec();
                    if let Some(register) = result {
                        let register = cx.register_for_write(register.index);
                        statements.push(air::Statement::RegisterWrite {
                            register,
                            value: packed,
                        });
                    }
                    let state_register = cx.register_for_write(state.id.index);
                    let merge_value = state_value_cx(cx, merge);
                    statements.push(air::Statement::RegisterWrite {
                        register: state_register,
                        value: merge_value,
                    });
                    air::Block {
                        statements: cx.alloc_slice(statements),
                    }
                }
            };
            let guard = elab_compare_inline(cx, state, current);
            let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
            cx.add_rule(air::Rule {
                name,
                guard: Some(guard),
                body: block,
            });
        });
        if ready {
            with_flow(|flow| flow.current = merge);
            return;
        }
        current = with_flow(|flow| flow.current);
    }
}

fn elab_compare_inline<'dsn>(
    cx: &mut Context<'dsn>,
    state: StateReg,
    index: usize,
) -> &'dsn air::Expr<'dsn> {
    let state_register = cx.register_handles[state.id.index].0;
    let state_ty = cx.ty::<U<32>>();
    let state_expr = cx.alloc(air::Expr {
        ty: state_ty,
        kind: air::ExprKind::Register(state_register),
    });
    let index_expr = state_value_cx(cx, index);
    let bool_ty = cx.ty::<bool>();
    cx.alloc(air::Expr {
        ty: bool_ty,
        kind: air::ExprKind::Binary {
            op: air::BinaryOp::Equal,
            lhs: state_expr,
            rhs: index_expr,
        },
    })
}

fn push_branch_cx<'dsn>(
    cx: &mut Context<'dsn>,
    block: air::Block<'dsn>,
    result: &'dsn air::Expr<'dsn>,
) -> super::BranchHandle {
    let handle = super::BranchHandle {
        design_id: cx.design_id,
        index: cx.branches.len(),
    };
    cx.branches.push(super::context::Branch {
        module: cx.id,
        parent_scope: cx.current_scope(),
        block,
        result,
    });
    handle
}
