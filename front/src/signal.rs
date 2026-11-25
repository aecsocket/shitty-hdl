use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use derive_more::Debug;
use pir::ShapeSize;

use crate::Shape;

#[derive(Debug)]
pub struct Signal<T: Shape> {
    id: SignalId,
    #[debug(skip)]
    view: T::View,
}

impl<T: Shape> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Signal<T> {}

impl<T: Shape> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Shape> Eq for Signal<T> {}

impl<T: Shape> Deref for Signal<T> {
    type Target = T::View;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<T: Shape> DerefMut for Signal<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
    }
}

impl<T: Shape> Signal<T> {
    #[must_use]
    pub fn new(init: impl Into<T>) -> Self {
        todo!();
    }

    #[must_use]
    pub fn without_init() -> Self {
        todo!();
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalId(u32);
