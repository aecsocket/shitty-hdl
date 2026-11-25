use std::{any::TypeId, marker::PhantomData, num::NonZero};

use crate::elab::{self, with_context};

/// A type-level clock domain brand.
///
/// Each marker type names one clock domain; mixing values of different
/// brands is rejected by the type system, and each brand must map to
/// exactly one clock (enforced during elaboration).
pub trait Domain: 'static {}

/// A clock in the domain `D`.
#[derive(Debug)]
pub struct Clock<D: Domain> {
    pub(crate) handle: Option<elab::ClockHandle>,
    pub hz: NonZero<u64>,
    _brand: PhantomData<D>,
}

impl<D: Domain> Clone for Clock<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain> Copy for Clock<D> {}

impl<D: Domain> Clock<D> {
    /// Declares the clock input of domain `D`. Elaboration asserts that
    /// `D` is only ever bound to one clock.
    #[must_use]
    pub fn input(name: &str, hz: NonZero<u64>) -> Self {
        Self {
            handle: Some(elab::clock_input::<D>(name)),
            hz,
            _brand: PhantomData,
        }
    }

    #[allow(clippy::unused_self)]
    pub(crate) const fn identity(&self) -> TypeId {
        TypeId::of::<D>()
    }

    /// Marks the enclosing module as using this clock, so that modules
    /// whose bodies contain no registers or IO still get one.
    pub(crate) fn ensure(&self) {
        if let Some(handle) = self.handle {
            with_context(|cx| {
                cx.check_design(handle.design_id);
                let domain = cx.domains[handle.index];
                cx.ensure_clock(domain);
            });
        }
    }
}
