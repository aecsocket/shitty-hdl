use std::ops::{BitAnd, BitOr, Not};

use crate::{
    air,
    domain::Domain,
    elab,
    reg::Reg,
    value::{IntoValue, Raw, Value},
};

impl Not for Raw<bool> {
    type Output = Self;

    fn not(self) -> Self::Output {
        elab::unary(air::UnaryOp::Not, self)
    }
}

impl BitOr<Raw<bool>> for Raw<bool> {
    type Output = Self;

    fn bitor(self, rhs: Raw<bool>) -> Self::Output {
        elab::binary(air::BinaryOp::Or, self, rhs)
    }
}

impl BitAnd<Raw<bool>> for Raw<bool> {
    type Output = Self;

    fn bitand(self, rhs: Raw<bool>) -> Self::Output {
        elab::binary(air::BinaryOp::And, self, rhs)
    }
}

impl<D: Domain> Not for Value<D, bool> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::wrap(!self.raw())
    }
}

impl<D: Domain, Rhs: IntoValue<D, Out = bool>> BitOr<Rhs> for Value<D, bool> {
    type Output = Self;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        Self::wrap(self.raw() | rhs.into_value().raw())
    }
}

impl<D: Domain, Rhs: IntoValue<D, Out = bool>> BitAnd<Rhs> for Value<D, bool> {
    type Output = Self;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        Self::wrap(self.raw() & rhs.into_value().raw())
    }
}

impl<D: Domain> Not for Reg<D, bool> {
    type Output = Value<D, bool>;

    fn not(self) -> Self::Output {
        !self.read()
    }
}

impl<D: Domain, Rhs: IntoValue<D, Out = bool>> BitOr<Rhs> for Reg<D, bool> {
    type Output = Value<D, bool>;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        self.read() | rhs
    }
}

impl<D: Domain, Rhs: IntoValue<D, Out = bool>> BitAnd<Rhs> for Reg<D, bool> {
    type Output = Value<D, bool>;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        self.read() & rhs
    }
}
