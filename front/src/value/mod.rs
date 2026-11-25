mod bit;
mod int;
mod prim;

pub use bit::*;
pub use int::*;

use std::marker::PhantomData;

use crate::domain::Domain;
use crate::elab::{self, Context, ExprHandle};

/// The type of a hardware value: how it is typed, laid out, and packed.
/// Domain-agnostic; the clock domain lives on [`Value`].
pub trait Shape: Sized + 'static {
    type View: Copy;
    type Values<D: Domain>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn>;
    fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn>;
    fn make_values<D: Domain>(value: Value<D, Self>) -> Self::Values<D>;
    fn pack_values<D: Domain>(values: Self::Values<D>) -> Value<D, Self>;
    fn value_handles<D: Domain>(values: Self::Values<D>) -> Vec<ExprHandle>;
}

pub type ShapeSize = u32;

/// An unbranded hardware expression. Internal representation used by the
/// elaboration machinery; the public API deals in domain-branded
/// [`Value`]s.
#[derive(Debug)]
pub struct Raw<T: Shape>(pub(crate) ExprHandle, PhantomData<T>);

impl<T: Shape> Raw<T> {
    #[must_use]
    pub const fn from_handle(handle: ExprHandle) -> Self {
        Self(handle, PhantomData)
    }

    #[must_use]
    pub const fn handle(self) -> ExprHandle {
        self.0
    }
}

impl<T: Shape> Clone for Raw<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Raw<T> {}

/// A hardware expression branded with the clock domain `D` it belongs
/// to. Values of different domains cannot be combined; crossing domains
/// requires an explicit synchronizer.
#[derive(Debug)]
pub struct Value<D: Domain, T: Shape> {
    raw: Raw<T>,
    _brand: PhantomData<D>,
}

impl<D: Domain, T: Shape> Value<D, T> {
    #[must_use]
    pub const fn wrap(raw: Raw<T>) -> Self {
        Self {
            raw,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub const fn raw(self) -> Raw<T> {
        self.raw
    }

    #[must_use]
    pub fn handle(self) -> ExprHandle {
        self.raw.handle()
    }
}

impl<D: Domain, T: Shape> Clone for Value<D, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, T: Shape> Copy for Value<D, T> {}

/// Coercion into a value of domain `D`. Constants (any `Shape` value)
/// coerce into any domain; branded values only coerce into their own.
///
/// Sealed: downstream crates cannot implement it (the extension point is
/// [`Shape`]), which keeps the operator impls coherent.
pub trait IntoValue<D: Domain>: private::Sealed {
    type Out: Shape;

    fn into_value(self) -> Value<D, Self::Out>;
}

mod private {
    use crate::{
        domain::Domain,
        value::{Raw, Shape},
    };

    pub trait Sealed {}

    impl<T: Shape> Sealed for T {}
    impl<T: Shape> Sealed for Raw<T> {}
    impl<D: Domain, T: Shape> Sealed for super::Value<D, T> {}
    impl<D: Domain> Sealed for crate::io::Output<D> {}
    impl<D: Domain, T: Shape> Sealed for crate::reg::Reg<D, T> {}
}

impl<D: Domain, T: Shape> IntoValue<D> for T {
    type Out = T;

    fn into_value(self) -> Value<D, T> {
        Value::wrap(elab::literal(self))
    }
}

impl<D: Domain, T: Shape> IntoValue<D> for Value<D, T> {
    type Out = T;

    fn into_value(self) -> Value<D, T> {
        self
    }
}

/// Coercion into the argument tuple of a shape, for method calls.
pub trait IntoValues<D: Domain> {
    type Out: Shape<Values<D> = Self::Values>;
    type Values;

    fn into_values(self) -> Self::Values;
}

impl<D: Domain, T: Shape> IntoValues<D> for T {
    type Out = T;
    type Values = <T as Shape>::Values<D>;

    fn into_values(self) -> Self::Values {
        T::make_values(self.into_value())
    }
}

impl<D: Domain, T: Shape> IntoValues<D> for Value<D, T> {
    type Out = T;
    type Values = <T as Shape>::Values<D>;

    fn into_values(self) -> Self::Values {
        T::make_values(self)
    }
}

#[derive(Debug)]
pub struct ValueView<T: Shape> {
    value: Raw<T>,
    offset: ShapeSize,
}

impl<T: Shape> Clone for ValueView<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for ValueView<T> {}
