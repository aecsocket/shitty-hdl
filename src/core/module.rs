use std::ops;

use crate::{IntoValue, Shape, Signal, Value};

pub trait Module {}

pub fn capture<T: Shape>(value: impl IntoValue<T>) -> Value<T> {
    todo!();
}

pub fn sync(f: impl FnOnce()) {}
pub fn comb(f: impl FnOnce()) {}

pub fn if_(cond: impl IntoValue<bool>, then: impl FnOnce()) {}

pub trait SignalFlow {
    type Inverse: SignalFlow;
}

#[derive(Clone, Copy)]
pub struct In<T: Shape>(Signal<T>);
#[derive(Clone, Copy)]
pub struct Out<T: Shape>(Signal<T>);

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
