use std::ops::{Deref, ShlAssign};

use crate::{
    domain::Domain,
    elab,
    value::{IntoValue, IntoValues, Shape, Value},
};

#[derive(Debug)]
pub struct Reg<T: Shape> {
    id: elab::RegisterHandle,
    value: Value<T>,
}

impl<T: Shape> Clone for Reg<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Reg<T> {}

impl<T: Shape> Reg<T> {
    #[must_use]
    pub fn new(domain: impl Domain, init: T) -> Self {
        let (id, value) = elab::register::<T>(domain.clock(), Some(init));
        Self { id, value }
    }

    #[must_use]
    pub fn uninit(domain: impl Domain) -> Self {
        let (id, value) = elab::register::<T>(domain.clock(), None);
        Self { id, value }
    }

    #[must_use]
    pub const fn read(self) -> Value<T> {
        self.value
    }

    pub fn write(&mut self, rhs: impl IntoValue<Out = T>) {
        elab::write_register(self.id, rhs.into_value());
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

impl<T: Shape<Values = Value<T>>> IntoValues for Reg<T> {
    type Out = T;
    type Values = Value<T>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for Reg<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}
