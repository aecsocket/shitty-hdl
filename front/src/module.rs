use std::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context as TaskContext, Poll},
    thread,
};

use crate::{
    air,
    domain::Domain,
    elab,
    elab::ExprHandle,
    value::{IntoValue, IntoValues, Shape, Value},
};

pub trait Interface: 'static {
    fn air_interface<'dsn>(cx: &mut elab::Context<'dsn>) -> &'dsn air::Interface<'dsn>;
    fn from_handle(handle: elab::InterfaceHandle) -> Self;
    fn air_bind<'dsn>(&self, cx: &mut elab::Context<'dsn>) -> &'dsn air::InterfaceImpl<'dsn>;

    #[must_use]
    fn input(name: &str) -> Self
    where
        Self: Sized,
    {
        elab::input_interface::<Self>(name)
    }
}

impl Interface for () {
    fn air_interface<'dsn>(cx: &mut elab::Context<'dsn>) -> &'dsn air::Interface<'dsn> {
        cx.alloc(air::Interface {
            name: "Empty",
            members: &[],
        })
    }

    fn from_handle(_handle: elab::InterfaceHandle) -> Self {}

    fn air_bind<'dsn>(&self, cx: &mut elab::Context<'dsn>) -> &'dsn air::InterfaceImpl<'dsn> {
        let ty = cx.iface::<Self>();
        cx.alloc(air::InterfaceImpl { ty, members: &[] })
    }
}

pub struct Rule(());

impl Rule {
    pub fn when<D: Domain>(cond: impl IntoValue<D, bool>) -> DefineRule {
        DefineRule {
            cond: cond.into_value().handle(),
        }
    }

    pub fn define_as(f: impl AsyncFnOnce(AsyncCx)) {
        elab::define_async_rule(None, f);
    }
}

pub struct DefineRule {
    cond: ExprHandle,
}

impl DefineRule {
    pub fn define_as(self, f: impl AsyncFnOnce(AsyncCx)) {
        elab::define_async_rule(Some(self.cond), f);
    }
}

pub struct Action<D: Domain, In: Shape, Out: Shape>(
    elab::ActionHandle,
    PhantomData<(D, (In, Out))>,
);

impl<D: Domain, In: Shape, Out: Shape> Action<D, In, Out> {
    #[must_use]
    pub const fn from_handle(handle: elab::ActionHandle) -> Self {
        Self(handle, PhantomData)
    }

    #[must_use]
    pub const fn handle(&self) -> elab::ActionHandle {
        self.0
    }

    #[must_use]
    pub fn from_interface(handle: elab::InterfaceHandle, name: &str) -> Self {
        Self::from_handle(elab::interface_action::<In, Out>(handle, name))
    }

    pub fn when(cond: impl IntoValue<D, bool>) -> DefineAction<D, In, Out> {
        DefineAction(Some(cond.into_value()), PhantomData)
    }

    pub fn define_as<O>(f: impl AsyncFnOnce(AsyncCx, Value<D, In>) -> O) -> Self
    where
        O: IntoValues<D, Values = Out::Values<D>>,
    {
        Self::from_handle(elab::define_action::<D, In, Out>(
            None,
            async move |cx, packed| f(cx, packed).await.into_values(),
        ))
    }

    pub fn call(
        &mut self,
        args: impl IntoValues<D, Out = In, Values = In::Values<D>>,
    ) -> Value<D, Out> {
        let output = elab::call_action::<D, In, Out>(self.0, args.into_values());
        Out::pack_values(output)
    }
}

pub struct DefineAction<D: Domain, In: Shape, Out: Shape>(
    Option<Value<D, bool>>,
    PhantomData<(D, (In, Out))>,
);

impl<D: Domain, In: Shape, Out: Shape> DefineAction<D, In, Out> {
    pub fn define_as<O>(self, f: impl AsyncFnOnce(AsyncCx, Value<D, In>) -> O) -> Action<D, In, Out>
    where
        O: IntoValues<D, Values = Out::Values<D>>,
    {
        let guard = self.0.map(super::value::Value::handle);
        Action::from_handle(elab::define_action::<D, In, Out>(
            guard,
            async move |cx, packed| f(cx, packed).await.into_values(),
        ))
    }
}

pub struct AsyncIf<'a, D: Domain, V: IntoValues<D>> {
    cond: Value<D, bool>,
    then: Option<Pin<Box<dyn Future<Output = V> + 'a>>>,
    else_: Option<Pin<Box<dyn Future<Output = V> + 'a>>>,
    result: Option<elab::RegisterHandle>,
    lowered: bool,
}

impl<'a, D: Domain, V: IntoValues<D>> AsyncIf<'a, D, V> {
    #[must_use]
    pub(crate) fn new(cond: Value<D, bool>, then: Pin<Box<dyn Future<Output = V> + 'a>>) -> Self {
        Self {
            cond,
            then: Some(then),
            else_: None,
            result: None,
            lowered: false,
        }
    }

    #[must_use]
    pub fn else_(mut self, else_: impl AsyncFnOnce() -> V + 'a) -> Self {
        self.else_ = Some(Box::pin(else_()));
        self
    }
}

impl<D: Domain, V: IntoValues<D>> Future for AsyncIf<'_, D, V> {
    type Output = V::Values;

    fn poll(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.lowered {
            return Poll::Ready(match this.result {
                Some(handle) => V::Out::make_values(elab::read_register::<D, V::Out>(handle)),
                None => V::Out::make_values(Value::<D, V::Out>::from_handle(elab::unit_value())),
            });
        }
        this.lowered = true;
        let then = this
            .then
            .take()
            .expect("async if branch was already lowered");
        let else_ = this
            .else_
            .take()
            .expect("async if branch was already lowered");
        match elab::lower_async_if(this.cond, then, else_) {
            elab::LoweredIf::Mux(value) => Poll::Ready(V::Out::make_values(value)),
            elab::LoweredIf::Fsm(result) => {
                this.result = result;
                Poll::Pending
            }
        }
    }
}

/// Token proving the current body is an async rule or method body.
///
/// Produced by `define_async_rule`/`define_async_method` and passed into
/// the body closure by the `hdl` macro; it cannot be constructed by user
/// code.
#[derive(Debug, Clone, Copy)]
pub struct AsyncCx(());

impl AsyncCx {
    pub(crate) const fn new() -> Self {
        Self(())
    }

    /// Two-way hardware branch inside an async rule or method body.
    ///
    /// The branches may suspend. In a sync rule or action, use `if`
    /// instead; the `AsyncCx` token only exists inside async bodies, so
    /// this cannot be used in sync context.
    #[must_use]
    pub fn if_<'a, D: Domain, V: IntoValues<D>>(
        self,
        cond: impl IntoValue<D, bool>,
        then: impl AsyncFnOnce() -> V + 'a,
    ) -> AsyncIf<'a, D, V> {
        AsyncIf::new(cond.into_value(), Box::pin(then()))
    }
}
