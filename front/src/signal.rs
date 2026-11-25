use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use derive_more::Debug;
use pir::ShapeSize;

use crate::{et::Brand, value::Shape};

#[derive(Debug)]
pub struct Signal<'d, T: Shape> {
    id: SignalId,
    #[debug(skip)]
    view: T::View<'d>,
    #[debug(skip)]
    _brand: Brand<'d>,
}

impl<T: Shape> Clone for Signal<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Signal<'_, T> {}

impl<T: Shape> PartialEq for Signal<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Shape> Eq for Signal<'_, T> {}

impl<'d, T: Shape> Deref for Signal<'d, T> {
    type Target = T::View<'d>;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<T: Shape> DerefMut for Signal<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
    }
}

impl<T: Shape> Signal<'_, T> {
    #[must_use]
    pub fn new(_init: impl Into<T>) -> Self {
        todo!();
    }

    #[must_use]
    pub fn without_init() -> Self {
        todo!();
    }
}

#[derive(Debug)]
pub struct SignalView<'d, T: Shape> {
    id: SignalId,
    offset: ShapeSize,
    #[debug(skip)]
    _phantom: PhantomData<T>,
    #[debug(skip)]
    _brand: Brand<'d>,
}

impl<T: Shape> Clone for SignalView<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for SignalView<'_, T> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalId(u32);
