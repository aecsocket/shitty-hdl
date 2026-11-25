use std::{marker::PhantomData, ops::ShlAssign};

use crate::{
    expr::IntoExpr,
    value::{Shape, ShapeSize},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(u32);

#[derive(Debug)]
pub struct Signal<T: Shape> {
    id: SignalId,
    view: T::View,
}

impl<T: Shape> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Signal<T> {}

impl<T: Shape> Signal<T> {
    #[must_use]
    pub fn new(init: impl IntoExpr<Out = T>) -> Self {
        todo!();
    }

    #[must_use]
    pub fn without_init() -> Self {
        todo!();
    }
}

/// Accessor to a slice of packed bits inside a [`Signal`].
///
/// The slice accessed by this view is defined as `offset..(offset + T::WIDTH)`.
#[derive(Debug)]
pub struct SignalView<T: Shape> {
    id: SignalId,
    offset: ShapeSize,
    _phantom: PhantomData<T>,
}

impl<T: Shape> Clone for SignalView<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for SignalView<T> {}

// read

impl<T: Shape> IntoExpr for Signal<T> {
    type Out = T;
}

impl<T: Shape> IntoExpr for SignalView<T> {
    type Out = T;
}

// write

impl<T: Shape, Rhs: IntoExpr<Out = T>> ShlAssign<Rhs> for Signal<T> {
    fn shl_assign(&mut self, rhs: Rhs) {}
}

impl<T: Shape, Rhs: IntoExpr<Out = T>> ShlAssign<Rhs> for SignalView<T> {
    fn shl_assign(&mut self, rhs: Rhs) {}
}
