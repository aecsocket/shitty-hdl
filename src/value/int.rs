use crate::{
    expr::{Expr, IntoExpr},
    signal::SignalView,
    util::ty::{Assert, True, fits_unsigned},
    value::{Shape, ShapeSize},
};

// un/signed ints

pub const fn cap_of<const N: ShapeSize>() -> usize {
    N.div_ceil(64) as usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uint<const N: ShapeSize>([u64; cap_of::<N>()])
where
    [(); cap_of::<N>()]:;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sint<const N: ShapeSize>([u64; cap_of::<N>()])
where
    [(); cap_of::<N>()]:;

// shape impls

impl<const N: ShapeSize> Shape for Uint<N>
where
    [(); cap_of::<N>()]:,
{
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

impl<const N: ShapeSize> Shape for Sint<N>
where
    [(); cap_of::<N>()]:,
{
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

// trait impls

pub trait Unsigned: Shape {}
impl<const N: ShapeSize> Unsigned for Uint<N> where [(); cap_of::<N>()]: {}

pub trait Signed: Shape {}
impl<const N: ShapeSize> Signed for Sint<N> where [(); cap_of::<N>()]: {}

// ctors

impl<const N: ShapeSize> Uint<N>
where
    [(); cap_of::<N>()]:,
{
    pub const fn constant<const VALUE: u64>() -> Self
    where
        Assert<{ fits_unsigned(N, VALUE) }>: True,
    {
        Self::from_words([VALUE; cap_of::<N>()])
    }
}

// ctor macros

#[macro_export]
macro_rules! u {
    ($value:expr $(,)?) => {{
        const VALUE: u64 = $value;
        $crate::value::Uint::constant::<VALUE>()
    }};
}

// from primitives

impl<const N: ShapeSize> Uint<N>
where
    [(); cap_of::<N>()]:,
{
    #[must_use]
    const fn from_words(words: [u64; cap_of::<N>()]) -> Self {
        Self(words)
    }
}

impl<const N: ShapeSize> Sint<N>
where
    [(); cap_of::<N>()]:,
{
    #[must_use]
    const fn from_words(words: [u64; cap_of::<N>()]) -> Self {
        Self(words)
    }
}

macro_rules! from_prim {
    (impl<N> From<$prim:ty> for $wrap:ident where N <= $width:literal |$in:ident| $cast:expr) => {
        impl<const N: ShapeSize> From<$prim> for $wrap<N>
        where
            [(); cap_of::<N>()]:,
            Assert<{ N >= $width }>: True,
        {
            fn from(value: $prim) -> Self {
                let out = (|$in: $prim| $cast)(value);
                Self::from_words([u64::from(out); cap_of::<N>()])
            }
        }

        impl IntoExpr for $prim {
            type Out = $wrap<$width>;

            fn into_expr(self) -> Expr<Self::Out> {
                Self::Out::from(self).into_expr()
            }
        }
    };
}

from_prim!(impl<N> From< u8> for Uint where N <= 8  |x| x);
from_prim!(impl<N> From<u16> for Uint where N <= 16 |x| x);
from_prim!(impl<N> From<u32> for Uint where N <= 32 |x| x);
from_prim!(impl<N> From<u64> for Uint where N <= 64 |x| x);
from_prim!(impl<N> From< i8> for Sint where N <= 8  |x| x.cast_unsigned());
from_prim!(impl<N> From<i16> for Sint where N <= 16 |x| x.cast_unsigned());
from_prim!(impl<N> From<i32> for Sint where N <= 32 |x| x.cast_unsigned());
from_prim!(impl<N> From<i64> for Sint where N <= 64 |x| x.cast_unsigned());
