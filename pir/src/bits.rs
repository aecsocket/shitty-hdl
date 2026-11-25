use smallvec::SmallVec;

use crate::ShapeSize;

#[derive(Debug, Clone)]
pub struct Bits {
    width: ShapeSize,
    words: SmallVec<u64, 2>,
}

impl Bits {
    pub fn into_parts(self) -> (ShapeSize, impl Iterator<Item = u64>) {
        (self.width, self.words.into_iter())
    }
}

impl From<bool> for Bits {
    fn from(value: bool) -> Self {
        let value = u64::from(value);
        Self {
            width: 1,
            words: SmallVec::from_buf([value]),
        }
    }
}

impl From<u128> for Bits {
    fn from(value: u128) -> Self {
        #[expect(clippy::cast_possible_truncation, reason = "on purpose")]
        let lo = value as u64;
        let hi = (value >> 64) as u64;
        Self {
            width: 128,
            words: SmallVec::from_buf([lo, hi]),
        }
    }
}

macro_rules! for_uint {
    ($($ty:ty: $width:literal),* $(,)?) => {
        $(
        impl From<$ty> for Bits {
            fn from(value: $ty) -> Self {
                Self {
                    width: $width,
                    words: SmallVec::from_buf([u64::from(value), 0]),
                }
            }
        }
        )*
    };
}

for_uint!(u64: 64, u32: 32, u16: 16, u8: 8);
