mod bits;
mod int;

pub use bits::*;
pub use int::*;

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

pub trait Shape: Sized {
    const WIDTH: ShapeSize;
    type View: Copy;
    const SIGNED: bool = false;

    fn to_bits(&self) -> crate::ir::PackedBits {
        panic!("implement Shape::to_bits or derive Shape for this type")
    }

    fn from_bits(_bits: &crate::ir::PackedBits) -> Self {
        panic!("implement Shape::from_bits or derive Shape for this type")
    }

    fn make_view(_id: crate::signal::SignalId, _offset: ShapeSize) -> Self::View {
        panic!("implement Shape::make_view or derive Shape for this type")
    }
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

    fn to_bits(&self) -> crate::ir::PackedBits {
        crate::ir::PackedBits::zero(<Self as Shape>::WIDTH)
    }

    fn from_bits(bits: &crate::ir::PackedBits) -> Self {
        assert_eq!(bits.width(), <Self as Shape>::WIDTH, "Shape width mismatch");
    }

    fn make_view(id: crate::signal::SignalId, offset: ShapeSize) -> Self::View {
        SignalView::new(id, offset)
    }
}

impl Shape for bool {
    const WIDTH: ShapeSize = 1;
    type View = SignalView<Self>;

    fn to_bits(&self) -> crate::ir::PackedBits {
        crate::ir::PackedBits::from_words(<Self as Shape>::WIDTH, vec![u64::from(*self)])
    }

    fn from_bits(bits: &crate::ir::PackedBits) -> Self {
        assert_eq!(bits.width(), <Self as Shape>::WIDTH, "Shape width mismatch");
        bits.words()[0] & 1 != 0
    }

    fn make_view(id: crate::signal::SignalId, offset: ShapeSize) -> Self::View {
        SignalView::new(id, offset)
    }
}
