use smallvec::SmallVec;

use crate::{
    expr::{Expr, IntoExpr},
    signal::SignalView,
    util::ty::{Assert, True, fits},
    value::{Shape, ShapeSize},
};

// un/signed ints

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct U<const N: ShapeSize>(SmallVec<u64, 1>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct I<const N: ShapeSize>(SmallVec<u64, 1>);

// shape impls

impl<const N: ShapeSize> Shape for U<N> {
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

impl<const N: ShapeSize> Shape for I<N> {
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

// trait impls

pub trait Unsigned: Shape {}
impl<const N: ShapeSize> Unsigned for U<N> {}

pub trait Signed: Shape {}
impl<const N: ShapeSize> Signed for I<N> {}

// ctors

impl<const N: ShapeSize> U<N> {
    pub const fn constant<const VALUE: u64>() -> Self
    where
        Assert<{ fits(N, VALUE) }>: True,
    {
        Self::from_word(VALUE)
    }
}

// ctor macros

#[macro_export]
macro_rules! u {
    ($value:expr $(,)?) => {
        $crate::value::U::constant::<{ $value }>()
    };
}

// from primitives

impl<const N: ShapeSize> U<N> {
    #[must_use]
    const fn from_word(word: u64) -> Self {
        Self(SmallVec::from_buf([word]))
    }
}

impl<const N: ShapeSize> I<N> {
    #[must_use]
    const fn from_word(word: u64) -> Self {
        Self(SmallVec::from_buf([word]))
    }
}

macro_rules! from_prim {
    (impl<N> From<$prim:ty> for $wrap:ident where N <= $width:literal |$in:ident| $cast:expr) => {
        impl<const N: ShapeSize> From<$prim> for $wrap<N>
        where
            Assert<{ N >= $width }>: True,
        {
            fn from(value: $prim) -> Self {
                let out = (|$in: $prim| $cast)(value);
                Self::from_word(u64::from(out))
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

from_prim!(impl<N> From< u8> for U where N <= 8  |x| x);
from_prim!(impl<N> From<u16> for U where N <= 16 |x| x);
from_prim!(impl<N> From<u32> for U where N <= 32 |x| x);
from_prim!(impl<N> From<u64> for U where N <= 64 |x| x);
from_prim!(impl<N> From< i8> for I where N <= 8  |x| x.cast_unsigned());
from_prim!(impl<N> From<i16> for I where N <= 16 |x| x.cast_unsigned());
from_prim!(impl<N> From<i32> for I where N <= 32 |x| x.cast_unsigned());
from_prim!(impl<N> From<i64> for I where N <= 64 |x| x.cast_unsigned());
