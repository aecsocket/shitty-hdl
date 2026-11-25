use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use crate::{
    air,
    domain::{Clock, Domain},
    elab,
    elab::RegisterHandle,
    module::{Action, AsyncCx, Interface},
    value::{IntoValues, Shape, Value},
};

pub struct Method<D: Domain, In: Shape, Out: Shape> {
    pub req: Action<D, In, ()>,
    pub out: Action<D, (), Out>,
}

impl<D: Domain, In: Shape, Out: Shape> Clone for Method<D, In, Out> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, In: Shape, Out: Shape> Copy for Method<D, In, Out> {}

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

    /// Defines a method in the given clock domain; the domain brands the
    /// method's interface, so calls and results are checked against it.
    /// The body may suspend with `.await`; a body that never suspends
    /// completes in a single cycle after the request.
    pub fn define_as<O>(clk: Clock<D>, f: impl AsyncFnOnce(AsyncCx, Value<D, In>) -> O) -> Self
    where
        O: IntoValues<D, Values = Out::Values<D>>,
    {
        clk.ensure();
        let (req, out) = elab::define_async_method::<D, In, Out, _>(async move |cx, packed| {
            f(cx, packed).await.into_values()
        });
        Self::new(req, out)
    }
}

pub struct Call<D: Domain, Out: IntoValues<D>> {
    issue: Option<Box<dyn FnOnce()>>,
    out: Action<D, (), Out::Out>,
    result: RegisterHandle,
    issued: bool,
}

impl<D: Domain, Out: IntoValues<D>> Call<D, Out> {
    #[must_use]
    pub fn new(out: Action<D, (), Out::Out>, issue: impl FnOnce() + 'static) -> Self
    where
        Out::Out: 'static,
    {
        Self {
            issue: Some(Box::new(issue)),
            out,
            result: elab::register_default::<D, Out::Out>(None).0,
            issued: false,
        }
    }
}

impl<D: Domain, Out: IntoValues<D>> Future for Call<D, Out> {
    type Output = Out::Values;

    fn poll(mut self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        // Three stages: issue the request, read the output, then suspend
        // once more before continuing the caller's body. The extra stage
        // keeps the output read in its own rule, so a body that awaits
        // the same method twice never issues one call's request in the
        // same rule as another call's output (their guards conflict).
        // The output value crosses the stage boundary through a register,
        // since expressions cannot escape their branch.
        if let Some(issue) = self.issue.take() {
            issue();
            self.issued = true;
            return Poll::Pending;
        }
        if self.issued {
            self.issued = false;
            let raw = elab::call_action::<D, (), Out::Out>(self.out.handle(), ());
            elab::write_register::<D, Out::Out>(self.result, <Out::Out as Shape>::pack_values(raw));
            return Poll::Pending;
        }
        Poll::Ready(<Out::Out as Shape>::make_values(elab::read_register::<
            D,
            Out::Out,
        >(self.result)))
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
        elab::call_action::<D, In, ()>(req, In::make_values(Value::from_handle(input)));
    })
}
