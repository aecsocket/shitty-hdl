use std::marker::PhantomData;
use std::ops::{Deref, ShlAssign};

use crate::{
    domain::{Clock, Domain},
    elab,
    value::{IntoValue, IntoValues, Shape, Value},
};

#[derive(Debug)]
pub struct Reg<D: Domain, T: Shape> {
    pub(crate) id: elab::RegisterHandle,
    value: Value<T>,
    _brand: PhantomData<D>,
}

impl<D: Domain, T: Shape> Clone for Reg<D, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, T: Shape> Copy for Reg<D, T> {}

impl<D: Domain, T: Shape> Reg<D, T> {
    #[must_use]
    pub fn new(domain: Clock<D>, init: T) -> Self {
        let (id, value) = elab::register(domain, Some(init));
        Self {
            id,
            value,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub fn uninit(domain: Clock<D>) -> Self {
        let (id, value) = elab::register(domain, None);
        Self {
            id,
            value,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub const fn read(self) -> Value<T> {
        self.value
    }

    pub fn write(&mut self, rhs: impl IntoValue<Out = T>) {
        elab::write_register(self.id, rhs.into_value());
    }
}

impl<D: Domain, T: Shape> Deref for Reg<D, T> {
    type Target = Value<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<D: Domain, T: Shape> IntoValue for Reg<D, T> {
    type Out = T;

    fn into_value(self) -> Value<Self::Out> {
        self.read()
    }
}

impl<D: Domain, T: Shape<Values = Value<T>>> IntoValues for Reg<D, T> {
    type Out = T;
    type Values = Value<T>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<D: Domain, T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for Reg<D, T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}
