use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, ShlAssign};

use crate::{
    domain::Domain,
    elab,
    value::{IntoValue, IntoValues, Shape, Value},
};

#[derive(Debug)]
pub struct Capture<D: Domain, T: Shape> {
    pub(crate) id: elab::RegisterHandle,
    value: Value<D, T>,
    _brand: PhantomData<D>,
}

impl<D: Domain, T: Shape> Clone for Capture<D, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, T: Shape> Copy for Capture<D, T> {}

impl<D: Domain, T: Shape> Capture<D, T> {
    #[must_use]
    pub fn new(value: impl IntoValue<D, T>) -> Self {
        let capture = Self::uninit();
        capture.write(value);
        capture
    }

    #[must_use]
    pub fn uninit() -> Self {
        let (id, _) = elab::register_default::<D, T>(None);
        Self {
            id,
            value: elab::read_register_module::<D, T>(id),
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub const fn read(self) -> Value<D, T> {
        self.value
    }

    pub fn write(self, rhs: impl IntoValue<D, T>) {
        elab::write_register(self.id, rhs.into_value());
    }
}

impl<D: Domain, T: Shape> Deref for Capture<D, T> {
    type Target = Value<D, T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<D: Domain, T: Shape> DerefMut for Capture<D, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<D: Domain, T: Shape> IntoValue<D, T> for Capture<D, T> {
    fn into_value(self) -> Value<D, T> {
        self.read()
    }
}

impl<D: Domain, T: Shape<Values<D> = Value<D, T>>> IntoValues<D> for Capture<D, T> {
    type Out = T;
    type Values = Value<D, T>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<D: Domain, T: Shape, Rhs: IntoValue<D, T>> ShlAssign<Rhs> for Capture<D, T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}
