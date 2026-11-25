use std::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context as TaskContext, Poll},
    thread,
};

use crate::{
    air, elab,
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
    pub fn when(cond: impl IntoValue<Out = bool>) -> DefineRule {
        DefineRule {
            cond: cond.into_value(),
        }
    }

    pub fn define_as(f: impl FnOnce()) {
        elab::define_rule(None, f);
    }

    pub fn define_async_as<'a>(f: impl AsyncFnOnce() + 'a) {
        elab::define_async_rule(f);
    }
}

pub struct DefineRule {
    cond: Value<bool>,
}

impl DefineRule {
    pub fn define_as(self, f: impl FnOnce()) {
        elab::define_rule(Some(self.cond), f);
    }
}

pub struct Action<In: Shape, Out: Shape>(elab::ActionHandle, PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> Action<In, Out> {
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

    pub fn when(cond: impl IntoValue<Out = bool>) -> DefineAction<In, Out> {
        DefineAction(Some(cond.into_value()), PhantomData)
    }

    pub fn define_as(f: impl FnOnce(In::Values) -> Out::Values) -> Self {
        Self::from_handle(elab::define_action::<In, Out>(None, f))
    }

    pub fn call(&mut self, args: impl IntoValues<Values = In::Values>) -> Out::Values {
        elab::call_action::<In, Out>(self.0, args.into_values())
    }
}

pub struct DefineAction<In: Shape, Out: Shape>(Option<Value<bool>>, PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> DefineAction<In, Out> {
    pub fn define_as(self, f: impl FnOnce(In::Values) -> Out::Values) -> Action<In, Out> {
        Action::from_handle(elab::define_action::<In, Out>(self.0, f))
    }
}

pub fn if_<R: IntoValues>(
    cond: impl IntoValue<Out = bool>,
    then: impl AsyncFnOnce() -> R,
) -> If<R> {
    let cond = elab::capture_branch(|| cond.into_value());
    let branch = elab::capture_branch(|| elab::run_ready(then()));
    If {
        cases: vec![(cond, branch)],
        _phantom: PhantomData,
    }
}

pub struct If<R: IntoValues> {
    cases: Vec<(elab::BranchHandle, elab::BranchHandle)>,
    _phantom: PhantomData<R>,
}

impl<R: IntoValues> If<R> {
    #[must_use]
    pub fn else_if(
        self,
        cond: impl FnOnce() -> Value<bool>,
        then: impl AsyncFnOnce() -> R,
    ) -> Self {
        self.else_if_cond(cond).then(then)
    }

    #[must_use]
    pub fn else_if_cond(self, cond: impl FnOnce() -> Value<bool>) -> ElseIf<R> {
        ElseIf {
            chain: self,
            cond: elab::capture_branch(cond),
        }
    }

    pub fn else_(mut self, then: impl AsyncFnOnce() -> R) -> R::Values {
        let branch = elab::capture_branch(|| elab::run_ready(then()));
        let value = elab::finish_if::<R::Out>(mem::take(&mut self.cases), Some(branch));
        R::Out::make_values(value)
    }
}

pub struct ElseIf<R: IntoValues> {
    chain: If<R>,
    cond: elab::BranchHandle,
}

impl<R: IntoValues> ElseIf<R> {
    #[must_use]
    pub fn then(mut self, then: impl AsyncFnOnce() -> R) -> If<R> {
        let branch = elab::capture_branch(|| elab::run_ready(then()));
        self.chain.cases.push((self.cond, branch));
        self.chain
    }
}

impl<R: IntoValues> Drop for If<R> {
    fn drop(&mut self) {
        if !thread::panicking() && !self.cases.is_empty() {
            let _ = elab::finish_if::<R::Out>(mem::take(&mut self.cases), None);
        }
    }
}

pub struct AsyncIf<'a, R: IntoValues> {
    cond: Value<bool>,
    then: Option<Pin<Box<dyn Future<Output = R> + 'a>>>,
    else_: Option<Pin<Box<dyn Future<Output = R> + 'a>>>,
    result: Option<elab::RegisterHandle>,
    lowered: bool,
    phantom: PhantomData<fn() -> R>,
}

impl<'a, R: IntoValues> AsyncIf<'a, R> {
    #[must_use]
    pub fn new(
        cond: impl IntoValue<Out = bool>,
        then: impl AsyncFnOnce() -> R + 'a,
        else_: impl AsyncFnOnce() -> R + 'a,
    ) -> Self {
        Self {
            cond: cond.into_value(),
            then: Some(Box::pin(then())),
            else_: Some(Box::pin(else_())),
            result: None,
            lowered: false,
            phantom: PhantomData,
        }
    }
}

impl<R: IntoValues> Future for AsyncIf<'_, R> {
    type Output = R::Values;

    fn poll(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.lowered {
            return Poll::Ready(match this.result {
                Some(handle) => R::Out::make_values(elab::read_register::<R::Out>(handle)),
                None => R::Out::make_values(Value::from_handle(elab::unit_value())),
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
        this.result = elab::lower_async_if(this.cond, then, else_);
        Poll::Pending
    }
}

impl<R: IntoValues> Unpin for AsyncIf<'_, R> {}
