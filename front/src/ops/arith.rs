use std::ops::{Add, Sub};

use crate::{
    ops::Bits,
    domain::Domain,
    reg::Reg,
    value::{IntoValue, Shape, Value},
};

pub trait Arith: Bits {
    fn add(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self>;

    fn sub(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self>;

    fn from_u128(value: u128) -> Value<Self>;
}

impl<T: Arith, Rhs: IntoValue<Out = T>> Add<Rhs> for Value<T> {
    type Output = Self;

    fn add(self, rhs: Rhs) -> Self::Output {
        T::add(self, rhs.into_value())
    }
}

impl<T: Arith> Add<u128> for Value<T> {
    type Output = Self;

    fn add(self, rhs: u128) -> Self::Output {
        T::add(self, T::from_u128(rhs))
    }
}

impl<T: Arith, Rhs: IntoValue<Out = T>> Sub<Rhs> for Value<T> {
    type Output = Self;

    fn sub(self, rhs: Rhs) -> Self::Output {
        T::sub(self, rhs.into_value())
    }
}

impl<T: Arith> Sub<u128> for Value<T> {
    type Output = Self;

    fn sub(self, rhs: u128) -> Self::Output {
        T::sub(self, T::from_u128(rhs))
    }
}

impl<D: Domain, T: Shape, Rhs> Add<Rhs> for Reg<D, T>
where
    Value<T>: Add<Rhs>,
{
    type Output = <Value<T> as Add<Rhs>>::Output;

    fn add(self, rhs: Rhs) -> Self::Output {
        self.read() + rhs
    }
}

impl<D: Domain, T: Shape, Rhs> Sub<Rhs> for Reg<D, T>
where
    Value<T>: Sub<Rhs>,
{
    type Output = <Value<T> as Sub<Rhs>>::Output;

    fn sub(self, rhs: Rhs) -> Self::Output {
        self.read() - rhs
    }
}
