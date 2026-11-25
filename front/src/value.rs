use std::marker::PhantomData;

use derive_more::Debug;
use pir::{Bits, ShapeSize};

use crate::signal::{SignalId, SignalView};

pub trait Shape: Sized {
    type View: Copy;

    fn make_view(id: SignalId, offset: ShapeSize) -> Self::View;

    fn into_bits(self) -> Bits;
}

impl Shape for () {
    type View = SignalView<Self>;

    fn make_view(id: SignalId, offset: ShapeSize) -> Self::View {
        SignalView::from_slice(id, offset)
    }

    fn into_bits(self) -> Bits {
        Bits::zero()
    }
}

impl Shape for bool {
    type View = SignalView<Self>;

    fn make_view(id: SignalId, offset: ShapeSize) -> Self::View {
        SignalView::from_slice(id, offset)
    }

    fn into_bits(self) -> Bits {
        Bits::from(self)
    }
}

#[derive(Debug)]
pub struct Value<T> {
    #[debug(skip)]
    _phantom: PhantomData<T>,
}

pub trait IntoValue {
    type Out: Shape;
}
