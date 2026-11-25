use std::ops::{Deref, ShlAssign};

use crate::value::{IntoValue, Shape, Value};

pub struct Reg<T: Shape> {
    id: RegId,
    value: Value<T>,
}

impl<T: Shape> Clone for Reg<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Reg<T> {}

impl<T: Shape> Reg<T> {
    pub fn new(init: T) -> Self {
        todo!();
    }

    pub fn assign(&mut self, rhs: impl IntoValue<Out = T>) {
        todo!();
    }
}

impl<T: Shape> Deref for Reg<T> {
    type Target = Value<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Shape> IntoValue for Reg<T> {
    type Out = T;

    fn into_value(self) -> Value<Self::Out> {
        self.value
    }
}

impl<T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for Reg<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.assign(rhs);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegId(usize);
