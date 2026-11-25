use std::marker::PhantomData;

use derive_more::Debug;

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
    #[debug(skip)]
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

impl Signal<bool> {
    #[track_caller]
    pub(crate) fn physical(
        name: &str,
        direction: ir::PortDirection,
        inverted: bool,
    ) -> (Self, ir::SignalId) {
        let id = runtime::physical_signal(name, direction);
        let physical = Self {
            id,
            view: bool::make_view(id, 0),
        };
        let logical = if inverted {
            let logical = Self::allocate(
                ir::PackedBits::zero(1),
                Some(format!("{name}_logical")),
                None,
            );
            match direction {
                ir::PortDirection::Input => runtime::physical_invert(logical.id, physical.id),
                ir::PortDirection::Output => runtime::physical_invert(physical.id, logical.id),
            }
            logical
        } else {
            physical
        };
        (logical, id.raw)
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
