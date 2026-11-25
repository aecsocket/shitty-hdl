use std::{
    marker::PhantomData,
    ops::{self, Deref},
};

use crate::{IntoValue, Shape, Signal, Value};

pub trait Module {}

pub fn capture<T: Shape>(value: impl IntoValue<T>) -> Value<T> {
    todo!();
}

pub fn if_<R>(cond: impl IntoValue<bool>, then: impl FnOnce() -> R) -> If<R> {
    todo!();
}

pub struct If<R> {
    _phantom: PhantomData<R>,
}

impl<R: Shape> If<R> {
    pub fn into_result(self) -> Value<R> {
        todo!();
    }

    pub fn else_if(self, cond: impl IntoValue<bool>, then: impl FnOnce() -> R) -> Self {
        todo!();
    }

    pub fn or_else(self, then: impl FnOnce() -> R) -> Else<R> {
        todo!();
    }
}

pub struct Else<R> {
    _phantom: PhantomData<R>,
}

impl<R: Shape> Else<R> {
    pub fn into_result(self) -> Value<R> {
        todo!();
    }
}

pub trait SignalFlow {
    type Inverse: SignalFlow;
}

#[derive(Clone, Copy)]
pub struct In<T: Shape>(Signal<T>);
#[derive(Clone, Copy)]
pub struct Out<T: Shape> {
    decomposed: T::Decomposed,
}

impl<T: Shape> Deref for Out<T> {
    type Target = T::Decomposed;

    fn deref(&self) -> &Self::Target {
        &self.decomposed
    }
}

impl<T: Shape> SignalFlow for In<T> {
    type Inverse = Out<T>;
}
impl<T: Shape> SignalFlow for Out<T> {
    type Inverse = In<T>;
}

impl<T: Shape> IntoValue<T> for In<T> {
    fn into_value(self) -> Value<T> {
        todo!();
    }
}
impl<T: Shape> IntoValue<T> for Out<T> {
    fn into_value(self) -> Value<T> {
        todo!();
    }
}

impl<T: Shape, Rhs: IntoValue<T>> ops::ShlAssign<Rhs> for Out<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        todo!();
    }
}
