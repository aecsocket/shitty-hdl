use std::ops::{Shl, Shr};

use crate::value::{Uint, Value};

impl<const N: usize> Shl<usize> for Value<Uint<N>> {
    type Output = Self;

    fn shl(self, rhs: usize) -> Self::Output {
        let _ = (self, rhs);
        todo!();
    }
}

impl<const N: usize> Shr<usize> for Value<Uint<N>> {
    type Output = Self;

    fn shr(self, rhs: usize) -> Self::Output {
        let _ = (self, rhs);
        todo!();
    }
}

pub trait ShlZext<const N: usize> {
    fn shl_zext<const SHIFT: usize>(self) -> Value<Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:;
}

pub trait ShrTrunc<const N: usize> {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:;
}

impl<const N: usize> ShlZext<N> for Value<Uint<N>> {
    fn shl_zext<const SHIFT: usize>(self) -> Value<Uint<{ N + SHIFT }>>
    where
        [(); N + SHIFT]:,
    {
        let _ = self;
        todo!();
    }
}

impl<const N: usize> ShrTrunc<N> for Value<Uint<N>> {
    fn shr_trunc<const SHIFT: usize>(self) -> Value<Uint<{ N - SHIFT }>>
    where
        [(); N - SHIFT]:,
    {
        let _ = self;
        todo!();
    }
}
