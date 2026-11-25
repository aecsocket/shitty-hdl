use std::marker::PhantomData;

use crate::{
    ir::{self, DesignId, runtime, shape_type},
    value::{Shape, ShapeSize},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId {
    pub(crate) owner: DesignId,
    pub(crate) raw: ir::SignalId,
}

#[derive(Debug)]
pub struct Signal<T: Shape> {
    pub(crate) id: SignalId,
    pub(crate) view: T::View,
}

impl<T: Shape> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for Signal<T> {}

impl<T: Shape> Signal<T> {
    #[track_caller]
    pub(crate) fn port(name: &str, direction: ir::PortDirection) -> Self {
        Self::allocate(
            ir::PackedBits::zero(T::WIDTH),
            Some(name.into()),
            Some(direction),
        )
    }

    #[track_caller]
    pub(crate) fn allocate(
        init: ir::PackedBits,
        name: Option<String>,
        port: Option<ir::PortDirection>,
    ) -> Self {
        assert_eq!(
            init.width(),
            T::WIDTH,
            "Shape::to_bits returned an incorrect width"
        );
        let id = runtime::signal(shape_type::<T>(), init, name, port);
        Self {
            id,
            view: T::make_view(id, 0),
        }
    }
}

#[derive(Debug)]
pub struct SignalView<T: Shape> {
    pub(crate) id: SignalId,
    pub(crate) offset: ShapeSize,
    _phantom: PhantomData<T>,
}

impl<T: Shape> SignalView<T> {
    pub fn new(id: SignalId, offset: ShapeSize) -> Self {
        Self {
            id,
            offset,
            _phantom: PhantomData,
        }
    }
}

impl<T: Shape> Clone for SignalView<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for SignalView<T> {}
