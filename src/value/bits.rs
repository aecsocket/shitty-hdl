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

    fn to_bits(&self) -> crate::ir::PackedBits {
        crate::ir::PackedBits::from_words(<Self as Shape>::WIDTH, self.0.as_slice().to_vec())
    }

    fn from_bits(bits: &crate::ir::PackedBits) -> Self {
        assert_eq!(bits.width(), <Self as Shape>::WIDTH, "Shape width mismatch");
        let mut words = GenericArray::<u64, N::Words>::const_default();
        words.as_mut_slice().copy_from_slice(bits.words());
        Self(words)
    }

    fn make_view(id: crate::signal::SignalId, offset: ShapeSize) -> Self::View {
        SignalView::new(id, offset)
    }
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
