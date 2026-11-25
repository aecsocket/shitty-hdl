use std::ops::{Shl, Shr};

use crate::{
    elab,
    ops::Bits,
    reg::Reg,
    value::{Shape, Uint, Value},
};

pub trait Shift: Bits {
    fn shl(value: Value<Self>, amount: usize) -> Value<Self>;

    fn shr(value: Value<Self>, amount: usize) -> Value<Self>;
}

impl<T: Shift> Shl<usize> for Value<T> {
    type Output = Self;

    fn shl(self, rhs: usize) -> Self::Output {
        T::shl(self, rhs)
    }
}

impl<T: Shift> Shr<usize> for Value<T> {
    type Output = Self;

    fn shr(self, rhs: usize) -> Self::Output {
        T::shr(self, rhs)
    }
}

pub trait ShlZext<const N: usize>: 'static {
    fn shl_zext<const SHIFT: usize>(self) -> Value<Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:;
}

pub trait ShrTrunc<const N: usize>: 'static {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:;
}

impl<const N: usize> ShlZext<N> for Value<Uint<N>> {
    fn shl_zext<const SHIFT: usize>(self) -> Value<Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:,
    {
        elab::shift_resize::<N, { N + SHIFT }>(air::BinaryOp::ShiftLeft, self, SHIFT)
    }
}

impl<const N: usize> ShrTrunc<N> for Value<Uint<N>> {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:,
    {
        elab::shift_resize::<N, { N - SHIFT }>(air::BinaryOp::ShiftRight, self, SHIFT)
    }
}

impl<T: Shape, Rhs> Shl<Rhs> for Reg<T>
where
    Value<T>: Shl<Rhs>,
{
    type Output = <Value<T> as Shl<Rhs>>::Output;

    fn shl(self, rhs: Rhs) -> Self::Output {
        self.read() << rhs
    }
}

impl<T: Shape, Rhs> Shr<Rhs> for Reg<T>
where
    Value<T>: Shr<Rhs>,
{
    type Output = <Value<T> as Shr<Rhs>>::Output;

    fn shr(self, rhs: Rhs) -> Self::Output {
        self.read() >> rhs
    }
}
