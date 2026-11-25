use std::ops::{BitAnd, BitOr, Not};

use crate::{
    elab,
    domain::Domain,
    reg::Reg,
    value::{IntoValue, Shape, Value},
};

impl Not for Value<bool> {
    type Output = Self;

    fn not(self) -> Self::Output {
        elab::unary(air::UnaryOp::Not, self)
    }
}

impl<Rhs: IntoValue<Out = bool>> BitOr<Rhs> for Value<bool> {
    type Output = Self;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        elab::binary(air::BinaryOp::Or, self, rhs.into_value())
    }
}

impl<Rhs: IntoValue<Out = bool>> BitAnd<Rhs> for Value<bool> {
    type Output = Self;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        elab::binary(air::BinaryOp::And, self, rhs.into_value())
    }
}

impl<D: Domain, T: Shape> Not for Reg<D, T>
where
    Value<T>: Not,
{
    type Output = <Value<T> as Not>::Output;

    fn not(self) -> Self::Output {
        !self.read()
    }
}

impl<D: Domain, T: Shape, Rhs> BitOr<Rhs> for Reg<D, T>
where
    Value<T>: BitOr<Rhs>,
{
    type Output = <Value<T> as BitOr<Rhs>>::Output;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        self.read() | rhs
    }
}

impl<D: Domain, T: Shape, Rhs> BitAnd<Rhs> for Reg<D, T>
where
    Value<T>: BitAnd<Rhs>,
{
    type Output = <Value<T> as BitAnd<Rhs>>::Output;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        self.read() & rhs
    }
}
