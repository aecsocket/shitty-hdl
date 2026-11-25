use smallvec::SmallVec;

use crate::{
    signal::SignalView,
    value::{Shape, ShapeSize},
};

/// `N` number of bits packed into one [`Shape`].
///
/// [`Bits`] has no arithmetic semantics, so it can only represent boolean data
/// such as a set of flags. To perform arithmetic such as addition or
/// subtraction, use [`Uint`] or [`Sint`].
pub struct Bits<const N: ShapeSize>(SmallVec<u64, 1>);

impl<const N: ShapeSize> Shape for Bits<N> {
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

impl<const N: ShapeSize> Bits<N> {
    fn from_word(word: u64) -> Self {
        Self(SmallVec::from_buf([word]))
    }

    fn from_words(words: impl IntoIterator<Item = u64>) -> Self {
        Self(SmallVec::from_iter(words))
    }
}

macro_rules! impl_from {
    ($($ty:ty : $size:literal),* $(,)?) => {
        $(
        impl From<$ty> for Bits<$size> {
            fn from(value: $ty) -> Self {
                Self::from_word(u64::from(value))
            }
        }
        )*
    };
}

impl_from!(u8: 8, u16: 16, u32: 32, u64: 64);

impl From<u128> for Bits<128> {
    fn from(value: u128) -> Self {
        #[rustfmt::skip]
        let [
            b1,  b2,  b3,  b4,  b5,  b6,  b7,  b8,
            b9, b10, b11, b12, b13, b14, b15, b16,
        ] = value.to_be_bytes();
        let (lo, hi) = (
            u64::from_be_bytes([b1, b2, b3, b4, b5, b6, b7, b8]),
            u64::from_be_bytes([b9, b10, b11, b12, b13, b14, b15, b16]),
        );
        Self::from_words([lo, hi])
    }
}
