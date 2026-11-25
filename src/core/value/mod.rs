mod cmp;
mod num;

pub use {cmp::*, num::*};

use std::marker::PhantomData;

pub trait Shape: Clone + Copy {}
impl Shape for bool {}

pub struct Value<T: Shape> {
    _phantom: PhantomData<T>,
}

pub trait IntoValue<T: Shape> {
    fn into_value(self) -> Value<T>;
}

impl<T: Shape> IntoValue<T> for T {
    fn into_value(self) -> Value<T> {
        todo!();
    }
}

impl<T: Shape> IntoValue<T> for Value<T> {
    fn into_value(self) -> Value<T> {
        self
    }
}
