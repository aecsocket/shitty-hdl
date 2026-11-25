use generic_array::GenericArray;
use typenum::{U8, U16, U32, U64, U128};

use crate::{
    signal::SignalView,
    value::{Shape, ShapeSize, Width},
};

/// `N` bits packed into one [`Shape`], with no arithmetic semantics.
///
/// To perform arithmetic, use [`super::Uint`] or [`super::Sint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bits<N: Width>(GenericArray<u64, N::Words>);

impl<N: Width> Shape for Bits<N> {
    const WIDTH: ShapeSize = N::U32;
    type View = SignalView<Self>;
}

impl<N: Width> Bits<N> {
    pub const fn words(&self) -> &[u64] {
        self.0.as_slice()
    }
}

macro_rules! impl_from {
    ($($ty:ty : $size:ty),* $(,)?) => {
        $(
        impl From<$ty> for Bits<$size> {
            fn from(value: $ty) -> Self {
                let mut words = GenericArray::<u64, <$size as Width>::Words>::const_default();
                let value = value as u128;
                words[0] = value as u64;
                if words.len() > 1 {
                    words[1] = (value >> 64) as u64;
                }
                Self(words)
            }
        }
        )*
    };
}

impl_from!(u8: U8, u16: U16, u32: U32, u64: U64, u128: U128);
