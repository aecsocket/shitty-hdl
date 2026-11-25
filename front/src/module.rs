use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use crate::{
    air,
    domain::Domain,
    elab,
    elab::ExprHandle,
    ops::BitCmp,
    value::{IntoValue, IntoValues, Shape, U, Value},
};

/// Marks a condition, iterator, or match scrutinee as elaboration-time
/// (Rust) control flow rather than hardware control flow.
///
/// Inside `#[module]` bodies, `if`, `while`, `for`, and `match` normally
/// lower to hardware control flow. Wrapping the condition, iterable, or
/// scrutinee in `elab(...)` keeps that construct as ordinary Rust evaluated
/// during elaboration:
///
/// ```text
/// if elab(flag) { ... }
/// for i in elab(0..values.len()) { ... }
/// while elab(n < 4) { ... }
/// match elab(mode) { ... }
/// ```
///
/// Only the control-flow decision is elaboration-time; hardware control flow
/// inside the body is still lowered normally. The marker must wrap the whole
/// condition, iterator, or scrutinee, and `elab` is recognized by name inside
/// module bodies.
///
/// This is an identity function: it returns `value` unchanged.
#[must_use]
pub const fn elab<T>(value: T) -> T {
    value
}

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

impl<D: Domain, In: Shape, Out: Shape> Clone for Action<D, In, Out> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, In: Shape, Out: Shape> Copy for Action<D, In, Out> {}

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
        &self,
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

/// A hardware `while` loop inside an async rule or method body.
///
/// The body is elaborated once and re-entered every cycle the condition
/// holds; the enclosing body resumes after the loop exits. `break` and
/// `continue` are not supported, because the Rust control-flow target would
/// not survive the rewrite.
pub struct AsyncWhile<'a, D: Domain> {
    cond: Option<Value<D, bool>>,
    body: Option<Pin<Box<dyn Future<Output = ()> + 'a>>>,
    lowered: bool,
}

impl<D: Domain> Future for AsyncWhile<'_, D> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.lowered {
            return Poll::Ready(());
        }
        this.lowered = true;
        let cond = this
            .cond
            .take()
            .expect("async while condition was already lowered");
        let body = this
            .body
            .take()
            .expect("async while body was already lowered");
        elab::lower_async_while(cond, body);
        Poll::Pending
    }
}

/// Marker for the internal index of a `for _ in range` loop, which never
/// escapes to user code.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct LoopDomain;

impl Domain for LoopDomain {}

/// A hardware `for` loop over a range inside an async rule or method body.
///
/// The bounds are captured into registers on entry, so they behave like a
/// Rust range: the end is not re-evaluated as the loop runs.
pub struct AsyncFor<'a, D: Domain, T: Shape> {
    body: Option<Pin<Box<dyn Future<Output = ()> + 'a>>>,
    index: elab::RegisterHandle,
    index_value: Value<D, T>,
    start: Value<D, T>,
    bound: elab::RegisterHandle,
    bound_value: Value<D, T>,
    end: Value<D, T>,
    inclusive: bool,
    lowered: bool,
}

impl<D: Domain, T: Shape> Unpin for AsyncFor<'_, D, T> {}

impl<D: Domain, T: Shape + BitCmp> Future for AsyncFor<'_, D, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.lowered {
            return Poll::Ready(());
        }
        this.lowered = true;
        let body = this
            .body
            .take()
            .expect("async for body was already lowered");
        elab::lower_async_for::<D, T>(
            body,
            this.index,
            this.index_value,
            this.start,
            this.bound,
            this.bound_value,
            this.end,
            this.inclusive,
        );
        Poll::Pending
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

    /// Hardware `while` loop inside an async body. The condition is evaluated
    /// every cycle; the body re-executes each time it holds.
    #[must_use]
    pub fn while_<'a, D: Domain>(
        self,
        cond: impl IntoValue<D, bool>,
        body: impl AsyncFnOnce() -> () + 'a,
    ) -> AsyncWhile<'a, D> {
        AsyncWhile {
            cond: Some(cond.into_value()),
            body: Some(Box::pin(body())),
            lowered: false,
        }
    }

    /// Hardware half-open range loop: `for i in start..end`.
    #[must_use]
    pub fn for_<'a, D, T, Start, End>(
        self,
        start: Start,
        end: End,
        body: impl AsyncFnOnce(Value<D, T>) -> () + 'a,
    ) -> AsyncFor<'a, D, T>
    where
        D: Domain,
        T: Shape + BitCmp,
        Start: IntoValue<D, T>,
        End: IntoValue<D, T>,
    {
        Self::for_impl(start, end, false, body)
    }

    /// Hardware inclusive range loop: `for i in start..=end`.
    #[must_use]
    pub fn for_inclusive<'a, D, T, Start, End>(
        self,
        start: Start,
        end: End,
        body: impl AsyncFnOnce(Value<D, T>) -> () + 'a,
    ) -> AsyncFor<'a, D, T>
    where
        D: Domain,
        T: Shape + BitCmp,
        Start: IntoValue<D, T>,
        End: IntoValue<D, T>,
    {
        Self::for_impl(start, end, true, body)
    }

    /// Hardware range loop whose index is ignored: `for _ in start..end`.
    /// The index never escapes, so no domain needs to be inferred.
    #[must_use]
    pub fn repeat<'a, const N: usize, Start, End>(
        self,
        start: Start,
        end: End,
        body: impl AsyncFnOnce() -> () + 'a,
    ) -> AsyncFor<'a, LoopDomain, U<N>>
    where
        Start: IntoValue<LoopDomain, U<N>>,
        End: IntoValue<LoopDomain, U<N>>,
    {
        Self::for_impl::<LoopDomain, U<N>, _, _>(start, end, false, async move |_index| {
            body().await;
        })
    }

    /// Inclusive [`Self::repeat`].
    #[must_use]
    pub fn repeat_inclusive<'a, const N: usize, Start, End>(
        self,
        start: Start,
        end: End,
        body: impl AsyncFnOnce() -> () + 'a,
    ) -> AsyncFor<'a, LoopDomain, U<N>>
    where
        Start: IntoValue<LoopDomain, U<N>>,
        End: IntoValue<LoopDomain, U<N>>,
    {
        Self::for_impl::<LoopDomain, U<N>, _, _>(start, end, true, async move |_index| body().await)
    }

    fn for_impl<'a, D, T, Start, End>(
        start: Start,
        end: End,
        inclusive: bool,
        body: impl AsyncFnOnce(Value<D, T>) -> () + 'a,
    ) -> AsyncFor<'a, D, T>
    where
        D: Domain,
        T: Shape + BitCmp,
        Start: IntoValue<D, T>,
        End: IntoValue<D, T>,
    {
        let start = start.into_value();
        let end = end.into_value();
        let (index, _) = elab::register_default::<D, T>(None);
        let (bound, _) = elab::register_default::<D, T>(None);
        let index_value = elab::read_register_module::<D, T>(index);
        let bound_value = elab::read_register_module::<D, T>(bound);
        AsyncFor {
            body: Some(Box::pin(body(index_value))),
            index,
            index_value,
            start,
            bound,
            bound_value,
            end,
            inclusive,
            lowered: false,
        }
    }
}
