mod bit;
mod int;
mod prim;

pub use bit::*;
pub use int::*;

use std::marker::PhantomData;

use crate::elab::{self, Context, ExprHandle};

pub trait Shape: Sized + 'static + IntoValues<Out = Self> {
    type View: Copy;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn>;
    fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn>;
    fn make_values(value: Value<Self>) -> Self::Values;
    fn pack_values(values: Self::Values) -> Value<Self>;
}

pub trait IntoValues {
    type Out: Shape<Values = Self::Values>;
    type Values;

    fn into_values(self) -> Self::Values;
}

pub type ShapeSize = u32;

#[derive(Debug)]
pub struct Value<T: Shape>(ExprHandle, PhantomData<T>);

impl<T: Shape> Value<T> {
    #[must_use]
    pub const fn from_handle(handle: ExprHandle) -> Self {
        Self(handle, PhantomData)
    }

    #[must_use]
    pub const fn handle(self) -> ExprHandle {
        self.0
    }
}

impl<T: Shape> Clone for Value<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Value<T> {}

pub trait IntoValue {
    type Out: Shape;

    fn into_value(self) -> Value<Self::Out>;
}

impl<T: Shape> IntoValue for T {
    type Out = T;

    fn into_value(self) -> Value<Self::Out> {
        elab::literal(self)
    }
}

impl<T: Shape> IntoValue for Value<T> {
    type Out = T;

    fn into_value(self) -> Value<Self::Out> {
        self
    }
}

impl<T: Shape<Values = Self>> IntoValues for Value<T> {
    type Out = T;
    type Values = Self;

    fn into_values(self) -> Self::Values {
        self
    }
}

#[derive(Debug)]
pub struct ValueView<T: Shape> {
    value: Value<T>,
    offset: ShapeSize,
}

impl<T: Shape> Clone for ValueView<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for ValueView<T> {}
