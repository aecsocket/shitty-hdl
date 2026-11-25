use smallvec::SmallVec;

use crate::{
    signal::SignalView,
    value::{Shape, ShapeSize},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uint<const N: ShapeSize>(SmallVec<u64, 3>);

impl<const N: ShapeSize> Uint<N> {
    pub const ZERO: Self = Self(SmallVec::from_buf([0]));
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sint<const N: ShapeSize>(SmallVec<u64, 3>);

impl<const N: ShapeSize> Shape for Uint<N> {
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

impl<const N: ShapeSize> Shape for Sint<N> {
    const WIDTH: ShapeSize = N;
    type View = SignalView<Self>;
}

macro_rules! impl_for {
    ($($ty:ty : $size:literal),* $(,)?) => {
        $(
        impl Shape for $ty {
            const WIDTH: ShapeSize = $size;
            type View = SignalView<Self>;
        }
        )*
    };
}

impl_for!(
    u8: 8, u16: 16, u32: 32, u64: 64, u128: 128,
    i8: 8, i16: 16, i32: 32, i64: 64, i128: 128,
);
