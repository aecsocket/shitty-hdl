use std::ops::{Deref, ShlAssign};

use crate::value::{IntoValue, Shape, Value};

#[derive(Debug)]
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

    #[must_use]
    pub const fn read(self) -> Value<T> {
        self.value
    }

    pub fn write(&mut self, rhs: impl IntoValue<Out = T>) {
        _ = rhs.into_value();
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
        self.read()
    }
}

impl<T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for Reg<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegId(usize);
