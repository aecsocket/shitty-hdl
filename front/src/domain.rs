use std::{any::TypeId, marker::PhantomData};

use crate::elab;

/// A type-level clock domain brand.
///
/// Each marker type names one clock domain; mixing values of different
/// brands is rejected by the type system, and each brand must map to
/// exactly one clock (enforced during elaboration).
pub trait Domain: Copy + 'static {}

#[derive(Debug, Clone, Copy)]
pub struct DefaultDomain;

impl Domain for DefaultDomain {}

/// A clock in the domain `D`.
#[derive(Debug, Clone, Copy)]
pub struct Clock<D: Domain> {
    pub(crate) handle: Option<elab::ClockHandle>,
    _brand: PhantomData<D>,
}

impl<D: Domain> Clock<D> {
    /// Declares the clock input of domain `D`. Elaboration asserts that
    /// `D` is only ever bound to one clock.
    #[must_use]
    pub fn input(name: &str) -> Self {
        Self {
            handle: Some(elab::clock_input::<D>(name)),
            _brand: PhantomData,
        }
    }

    #[allow(clippy::unused_self)]
    pub(crate) const fn identity(&self) -> TypeId {
        TypeId::of::<D>()
    }
}

impl Default for Clock<DefaultDomain> {
    fn default() -> Self {
        Self {
            handle: None,
            _brand: PhantomData,
        }
    }
}
