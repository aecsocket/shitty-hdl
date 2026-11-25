mod bits;
mod int;

use crate::signal::SignalView;
use const_default::ConstDefault;
use generic_array::ArrayLength;
use std::ops::{Add, Div};
use typenum::{B1, IsLessOrEqual, Quot, Sum, U63, U64, U4294967295, Unsigned as TypeUnsigned};

pub trait Width: TypeUnsigned {
    type Words: ArrayLength<ArrayType<u64>: Copy + ConstDefault>;
}

impl<N> Width for N
where
    N: TypeUnsigned + IsLessOrEqual<U4294967295, Output = B1> + Add<U63>,
    Sum<N, U63>: Div<U64>,
    Quot<Sum<N, U63>, U64>: ArrayLength,
    <Quot<Sum<N, U63>, U64> as ArrayLength>::ArrayType<u64>: Copy + ConstDefault,
{
    type Words = Quot<Sum<N, U63>, U64>;
}
pub use {bits::*, int::*};

pub trait Shape: Sized {
    const WIDTH: ShapeSize;
    type View: Copy;
}

/// Represents the width of a [`Shape`], in bits.
///
/// [`ShapeSize::MAX`] is the maximum packed width of a single shape, which also
/// makes it the maximum packed width of a [signal].
///
/// [signal]: crate::signal::Signal
pub type ShapeSize = u32;

impl Shape for () {
    const WIDTH: ShapeSize = 0;
    type View = SignalView<Self>;
}

impl Shape for bool {
    const WIDTH: ShapeSize = 1;
    type View = SignalView<Self>;
}
