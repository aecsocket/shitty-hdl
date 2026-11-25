mod bits;
mod int;

use crate::signal::SignalView;
pub use {bits::*, int::*};

pub trait Shape {
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
