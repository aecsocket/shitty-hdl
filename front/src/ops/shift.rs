use std::ops::{Shl, Shr};

use crate::{
    air,
    domain::Domain,
    elab,
    ops::Bits,
    reg::Reg,
    value::{Shape, Uint, Value},
};

pub trait Shift: Bits {
    fn shl<D: Domain>(value: Value<D, Self>, amount: usize) -> Value<D, Self>;

    fn shr<D: Domain>(value: Value<D, Self>, amount: usize) -> Value<D, Self>;
}

impl<D: Domain, T: Shape + Shift> Shl<usize> for Value<D, T> {
    type Output = Self;

    fn shl(self, rhs: usize) -> Self::Output {
        T::shl(self, rhs)
    }
}

impl<D: Domain, T: Shape + Shift> Shr<usize> for Value<D, T> {
    type Output = Self;

    fn shr(self, rhs: usize) -> Self::Output {
        T::shr(self, rhs)
    }
}

pub trait ShlZext<const N: usize, D: Domain>: 'static {
    fn shl_zext<const SHIFT: usize>(self) -> Value<D, Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:;
}

pub trait ShrTrunc<const N: usize, D: Domain>: 'static {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<D, Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:;
}

impl<D: Domain, const N: usize> ShlZext<N, D> for Value<D, Uint<N>> {
    fn shl_zext<const SHIFT: usize>(self) -> Value<D, Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:,
    {
        elab::shift_resize(air::BinaryOp::ShiftLeft, self, SHIFT)
    }
}

impl<D: Domain, const N: usize> ShrTrunc<N, D> for Value<D, Uint<N>> {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<D, Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:,
    {
        elab::shift_resize(air::BinaryOp::ShiftRight, self, SHIFT)
    }
}

impl<D: Domain, T: Shape, Rhs> Shl<Rhs> for Reg<D, T>
where
    Value<D, T>: Shl<Rhs>,
{
    type Output = <Value<D, T> as Shl<Rhs>>::Output;

    fn shl(self, rhs: Rhs) -> Self::Output {
        self.read() << rhs
    }
}

impl<D: Domain, T: Shape, Rhs> Shr<Rhs> for Reg<D, T>
where
    Value<D, T>: Shr<Rhs>,
{
    type Output = <Value<D, T> as Shr<Rhs>>::Output;

    fn shr(self, rhs: Rhs) -> Self::Output {
        self.read() >> rhs
    }
}
