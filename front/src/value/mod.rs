mod bit;
mod int;
mod prim;

pub use bit::*;
pub use int::*;

use std::marker::PhantomData;

pub trait Shape: Sized {
    type View: Copy;
    type Values;
}

pub type ShapeSize = u32;

#[derive(Debug)]
pub struct Value<T: Shape>(PhantomData<T>);

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
        todo!();
    }
}

impl<T: Shape> IntoValue for Value<T> {
    type Out = T;

    fn into_value(self) -> Value<Self::Out> {
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
