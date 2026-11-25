use std::ops::{Add, Sub};

use crate::{
    domain::Domain,
    ops::Bits,
    reg::Reg,
    value::{IntoValue, Shape, Value},
};

pub trait Arith: Bits {
    fn add<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self>;

    fn sub<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self>;

    fn from_u128<D: Domain>(value: u128) -> Value<D, Self>;
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Add<Rhs> for Value<D, T> {
    type Output = Self;

    fn add(self, rhs: Rhs) -> Self::Output {
        T::add(self, rhs.into_value())
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Sub<Rhs> for Value<D, T> {
    type Output = Self;

    fn sub(self, rhs: Rhs) -> Self::Output {
        T::sub(self, rhs.into_value())
    }
}

impl<D: Domain, T: Shape + Arith> Add<u128> for Value<D, T> {
    type Output = Self;

    fn add(self, rhs: u128) -> Self::Output {
        T::add(self, T::from_u128(rhs))
    }
}

impl<D: Domain, T: Shape + Arith> Sub<u128> for Value<D, T> {
    type Output = Self;

    fn sub(self, rhs: u128) -> Self::Output {
        T::sub(self, T::from_u128(rhs))
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Add<Rhs> for Reg<D, T> {
    type Output = Value<D, T>;

    fn add(self, rhs: Rhs) -> Self::Output {
        self.read() + rhs
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Sub<Rhs> for Reg<D, T> {
    type Output = Value<D, T>;

    fn sub(self, rhs: Rhs) -> Self::Output {
        self.read() - rhs
    }
}

impl<D: Domain, T: Shape + Arith> Add<u128> for Reg<D, T> {
    type Output = Value<D, T>;

    fn add(self, rhs: u128) -> Self::Output {
        self.read() + rhs
    }
}

impl<D: Domain, T: Shape + Arith> Sub<u128> for Reg<D, T> {
    type Output = Value<D, T>;

    fn sub(self, rhs: u128) -> Self::Output {
        self.read() - rhs
    }
}
