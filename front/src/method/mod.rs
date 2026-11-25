use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use crate::{
    air, elab,
    module::{Action, Interface},
    value::{IntoValue, IntoValues, Shape, Value},
};

pub struct Method<In: Shape, Out: Shape> {
    pub req: Action<In, ()>,
    pub out: Action<(), Out>,
}

impl<In: Shape, Out: Shape> Interface for Method<In, Out> {
    fn air_interface<'dsn>(cx: &mut elab::Context<'dsn>) -> &'dsn air::Interface<'dsn> {
        cx.method_interface::<In, Out>()
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

impl<In: Shape, Out: Shape> Method<In, Out> {
    #[must_use]
    pub const fn new(req: Action<In, ()>, out: Action<(), Out>) -> Self {
        Self { req, out }
    }

    pub fn define_as<'a>(f: impl AsyncFnOnce(In::Values) -> Out::Values + 'a) -> Self {
        let (req, out) = elab::define_async_method(f);
        Self::new(req, out)
    }

    pub fn define_async_as<'a>(f: impl AsyncFnOnce(In::Values) -> Out::Values + 'a) -> Self {
        Self::define_as(f)
    }
}

pub struct Call<Out: Shape> {
    issue: Option<Box<dyn FnOnce()>>,
    out: Action<(), Out>,
}

impl<Out: Shape> Call<Out> {
    #[must_use]
    pub fn new(out: Action<(), Out>, issue: impl FnOnce() + 'static) -> Self {
        Self {
            issue: Some(Box::new(issue)),
            out,
        }
    }
}

impl<Out: Shape> Future for Call<Out> {
    type Output = Out::Values;

    fn poll(mut self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        match self.issue.take() {
            Some(issue) => {
                issue();
                Poll::Pending
            }
            None => Poll::Ready(elab::call_action::<(), Out>(self.out.handle(), ())),
        }
    }
}

impl<Out: Shape> Unpin for Call<Out> {}

pub fn call_async<In: Shape, Out: Shape>(
    req: &Action<In, ()>,
    out: &Action<(), Out>,
    args: &[elab::ExprHandle],
) -> Call<Out> {
    let input = elab::pack_input::<In>(args);
    let req = req.handle();
    let out = out.handle();
    Call::new(Action::from_handle(out), move || {
        elab::call_action::<In, ()>(req, In::make_values(Value::from_handle(input)));
    })
}
