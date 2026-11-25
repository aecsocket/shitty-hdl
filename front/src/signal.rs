use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use derive_more::Debug;
use pir::ShapeSize;

use crate::{
    elab::{self, DesignId},
    value::Shape,
};

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
    #[track_caller]
    pub fn new(init: impl Into<T>) -> Self {
        Self::with_maybe_init(Some(init.into()))
    }

    #[must_use]
    #[track_caller]
    pub fn without_init() -> Self {
        Self::with_maybe_init(None)
    }

    #[track_caller]
    fn with_maybe_init(init: Option<T>) -> Self {
        let id = elab::make_signal(init);
        Self {
            id,
            view: T::make_view(id, 0),
        }
    }
}

#[derive(Debug)]
pub struct SignalView<T: Shape> {
    id: SignalId,
    offset: ShapeSize,
    #[debug(skip)]
    _phantom: PhantomData<T>,
}

impl<T: Shape> Clone for SignalView<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for SignalView<T> {}

impl<T: Shape> SignalView<T> {
    #[must_use]
    pub const fn from_slice(id: SignalId, offset: ShapeSize) -> Self {
        Self {
            id,
            offset,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalId {
    pub(crate) design_id: DesignId,
    pub(crate) id: u32,
}
