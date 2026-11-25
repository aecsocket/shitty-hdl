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
    value: Value<D, T>,
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
    pub fn new(clk: Clock<D>, init: T) -> Self {
        let (id, value) = elab::register(clk, Some(init));
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
    pub const fn read(self) -> Value<D, T> {
        self.value
    }

    pub fn write(&mut self, rhs: impl IntoValue<D, T>) {
        elab::write_register(self.id, rhs.into_value());
    }
}

impl<D: Domain, T: Shape> Deref for Reg<D, T> {
    type Target = Value<D, T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<D: Domain, T: Shape> IntoValue<D, T> for Reg<D, T> {
    fn into_value(self) -> Value<D, T> {
        self.read()
    }
}

impl<D: Domain, T: Shape<Values<D> = Value<D, T>>> IntoValues<D> for Reg<D, T> {
    type Out = T;
    type Values = Value<D, T>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<D: Domain, T: Shape, Rhs: IntoValue<D, T>> ShlAssign<Rhs> for Reg<D, T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}
