use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use crate::{
    air,
    domain::{Clock, Domain, IntoClock},
    elab,
    module::{Action, Interface},
    value::{IntoValues, Raw, Shape, Value},
};

pub struct Method<D: Domain, In: Shape, Out: Shape> {
    pub req: Action<D, In, ()>,
    pub out: Action<D, (), Out>,
}

impl<D: Domain, In: Shape, Out: Shape> Interface for Method<D, In, Out> {
    fn air_interface<'dsn>(cx: &mut elab::Context<'dsn>) -> &'dsn air::Interface<'dsn> {
        cx.method_interface::<D, In, Out>()
    }

    fn from_handle(handle: elab::InterfaceHandle) -> Self {
        Self::new(
            Action::from_interface(handle, "req"),
            Action::from_interface(handle, "out"),
        )
    }

    fn air_bind<'dsn>(&self, cx: &mut elab::Context<'dsn>) -> &'dsn air::InterfaceImpl<'dsn> {
        let ty = cx.iface::<Self>();
        let handles = [self.req.handle(), self.out.handle()];
        let mut members = Vec::new();
        for (declaration, handle) in ty.members.iter().zip(handles) {
            let air::InterfaceMember::Method(declaration) = declaration else {
                panic!("expected method declaration");
            };
            members.push(air::InterfaceBinding::Method {
                declaration,
                method: cx.action_ref(handle),
            });
        }
        let members = cx.alloc_slice(members);
        cx.alloc(air::InterfaceImpl { ty, members })
    }
}

impl<D: Domain, In: Shape, Out: Shape> Method<D, In, Out> {
    #[must_use]
    pub const fn new(req: Action<D, In, ()>, out: Action<D, (), Out>) -> Self {
        Self { req, out }
    }

    /// Defines an async method in the given clock domain; the domain
    /// brands the method's interface, so calls and results are checked
    /// against it.
    pub fn define_as<'a>(
        clock: Clock<D>,
        f: impl AsyncFnOnce(In::Values<D>) -> Out::Values<D> + 'a,
    ) -> Self {
        let _ = clock;
        let (req, out) = elab::define_async_method::<D, In, Out, _>(f);
        Self::new(req, out)
    }

    pub fn define_async_as<'a>(
        clock: Clock<D>,
        f: impl AsyncFnOnce(In::Values<D>) -> Out::Values<D> + 'a,
    ) -> Self {
        Self::define_as(clock, f)
    }
}

pub struct Call<D: Domain, Out: IntoValues<D>> {
    issue: Option<Box<dyn FnOnce()>>,
    out: Action<D, (), Out::Out>,
}

impl<D: Domain, Out: IntoValues<D>> Call<D, Out> {
    #[must_use]
    pub fn new(out: Action<D, (), Out::Out>, issue: impl FnOnce() + 'static) -> Self {
        Self {
            issue: Some(Box::new(issue)),
            out,
        }
    }
}

impl<D: Domain, Out: IntoValues<D>> Future for Call<D, Out> {
    type Output = Out::Values;

    fn poll(mut self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        match self.issue.take() {
            Some(issue) => {
                issue();
                Poll::Pending
            }
            None => {
                let raw = elab::call_action::<D, (), Out::Out>(self.out.handle(), ());
                Poll::Ready(raw)
            }
        }
    }
}

impl<D: Domain, Out: IntoValues<D>> Unpin for Call<D, Out> {}

#[must_use]
pub fn call_async<D: Domain, In: Shape, Out: Shape + IntoValues<D>>(
    req: &Action<D, In, ()>,
    out: &Action<D, (), Out>,
    args: &[elab::ExprHandle],
) -> Call<D, Out> {
    let input = elab::pack_input::<In>(args);
    let req = req.handle();
    let out = out.handle();
    Call::new(Action::from_handle(out), move || {
        elab::call_action::<D, In, ()>(req, In::make_values(Value::wrap(Raw::from_handle(input))));
    })
}
