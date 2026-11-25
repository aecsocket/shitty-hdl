use std::marker::PhantomData;
use std::ops::{Deref, ShlAssign};

use crate::{
    domain::{Domain, IntoClock},
    elab,
    ops::Arith,
    value::{IntoValue, IntoValues, Shape, Value},
};

pub trait IntoRegValue<D: Domain, T: Shape> {
    fn into_reg_value(self) -> Value<D, T>;
}

impl<D: Domain, T: Shape, Rhs: IntoValue<D, Out = T>> IntoRegValue<D, T> for Rhs {
    fn into_reg_value(self) -> Value<D, T> {
        self.into_value()
    }
}

impl<D: Domain, T: Shape + Arith> IntoRegValue<D, T> for u128 {
    fn into_reg_value(self) -> Value<D, T> {
        T::from_u128(self)
    }
}

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
    pub fn new(domain: impl IntoClock<D>, init: T) -> Self {
        let (id, value) = elab::register(domain.into_clock(), Some(init));
        Self {
            id,
            value,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub fn uninit(domain: impl IntoClock<D>) -> Self {
        let (id, value) = elab::register(domain.into_clock(), None);
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

    pub fn write(&mut self, rhs: impl IntoRegValue<D, T>) {
        elab::write_register(self.id, rhs.into_reg_value());
    }
}

impl<D: Domain, T: Shape> Deref for Reg<D, T> {
    type Target = Value<D, T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<D: Domain, T: Shape> IntoValue<D> for Reg<D, T> {
    type Out = T;

    fn into_value(self) -> Value<D, Self::Out> {
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

impl<D: Domain, T: Shape, Rhs: IntoRegValue<D, T>> ShlAssign<Rhs> for Reg<D, T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}
