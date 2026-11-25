use std::marker::PhantomData;

use crate::value::{IntoValue, Shape, Value};

pub trait Interface {}

pub struct Method<In: Shape, Out: Shape>(PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> Method<In, Out> {
    pub fn define_as(f: impl AsyncFnOnce(In::Values) -> Out::Values) -> Self {
        drop(f);
        todo!();
    }
}

pub fn if_<R: IntoValue>(cond: impl IntoValue<Out = bool>, then: impl AsyncFnOnce() -> R) -> If<R> {
    drop((cond, then));
    todo!();
}

pub struct If<T> {
    _phantom: PhantomData<T>,
}

impl<T: IntoValue> If<T> {
    #[must_use]
    pub fn else_if(self, cond: impl IntoValue<Out = bool>, then: impl AsyncFnOnce() -> T) -> Self {
        drop((cond, then));
        todo!();
    }

    pub fn else_(self, then: impl AsyncFnOnce() -> T) -> Value<T::Out> {
        drop(then);
        todo!();
    }
}
