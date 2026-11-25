use crate::{
    expr::{Expr, IntoExpr},
    signal::SignalView,
    util::ty::fits_unsigned,
    value::{Shape, ShapeSize, Width},
};
use generic_array::GenericArray;
use typenum::{B1, IsGreaterOrEqual, U8, U16, U32, U64, U128};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uint<N: Width>(GenericArray<u64, N::Words>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sint<N: Width>(GenericArray<u64, N::Words>);

impl<N: Width> Shape for Uint<N> {
    const WIDTH: ShapeSize = N::U32;
    type View = SignalView<Self>;
}

impl<N: Width> Shape for Sint<N> {
    const WIDTH: ShapeSize = N::U32;
    type View = SignalView<Self>;
}

pub trait Unsigned: Shape {}
impl<N: Width> Unsigned for Uint<N> {}

pub trait Signed: Shape {}
impl<N: Width> Signed for Sint<N> {}

impl<N: Width> Uint<N> {
    pub const fn constant<const VALUE: u64>() -> Self {
        const {
            assert!(
                fits_unsigned(N::U32, VALUE),
                "unsigned literal does not fit its width"
            );
        }
        let mut words = GenericArray::<u64, N::Words>::const_default();
        if N::U32 != 0 {
            words.as_mut_slice()[0] = VALUE;
        }
        Self(words)
    }

    pub const fn words(&self) -> &[u64] {
        self.0.as_slice()
    }
}

impl<N: Width> Sint<N> {
    pub const fn words(&self) -> &[u64] {
        self.0.as_slice()
    }
}

#[macro_export]
macro_rules! u {
    ($value:expr $(,)?) => {{
        const VALUE: u64 = $value;
        const { $crate::value::Uint::constant::<VALUE>() }
    }};
}

macro_rules! from_prim {
    ($prim:ty, $wrap:ident, $width:ty, $signed:expr) => {
        impl<N: Width + IsGreaterOrEqual<$width, Output = B1>> From<$prim> for $wrap<N> {
            fn from(value: $prim) -> Self {
                let mut words = GenericArray::<u64, N::Words>::const_default();
                // Casting through i128 sign-extends signed inputs before splitting words.
                let bits = if $signed {
                    value as i128 as u128
                } else {
                    value as u128
                };
                let extension = if $signed && (value as i128) < 0 {
                    u64::MAX
                } else {
                    0
                };
                for (index, word) in words.iter_mut().enumerate() {
                    *word = match index {
                        0 => bits as u64,
                        1 => (bits >> 64) as u64,
                        _ => extension,
                    };
                }
                // Unused high bits are always zero, including for signed values.
                let remainder = N::U32 % 64;
                if remainder != 0 {
                    let last = words.len() - 1;
                    words[last] &= (1u64 << remainder) - 1;
                }
                Self(words)
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

from_prim!(u8, Uint, U8, false);
from_prim!(u16, Uint, U16, false);
from_prim!(u32, Uint, U32, false);
from_prim!(u64, Uint, U64, false);
from_prim!(u128, Uint, U128, false);
from_prim!(i8, Sint, U8, true);
from_prim!(i16, Sint, U16, true);
from_prim!(i32, Sint, U32, true);
from_prim!(i64, Sint, U64, true);
from_prim!(i128, Sint, U128, true);
