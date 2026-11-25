mod bit;
mod int;
mod prim;
mod vector;

pub use bit::*;
pub use int::*;
pub use vector::*;

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, ShlAssign};

use crate::capture::Capture;
use crate::domain::Domain;
use crate::elab::{self, Context, ExprHandle};
use crate::module::LoopDomain;
use crate::ops::Arith;
use crate::reg::Reg;

/// The type of a hardware value: how it is typed, laid out, and packed.
/// Domain-agnostic; the clock domain lives on [`Value`].
pub trait Shape: Sized + 'static {
    type View<D: Domain>: Copy;
    type Values<D: Domain>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn>;
    fn air_value<'dsn>(&self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn>;
    fn make_view<D: Domain>(handle: ExprHandle) -> Self::View<D>;
    fn make_values<D: Domain>(value: Value<D, Self>) -> Self::Values<D>;
    fn pack_values<D: Domain>(values: Self::Values<D>) -> Value<D, Self>;
    fn value_handles<D: Domain>(values: Self::Values<D>) -> Vec<ExprHandle>;
}

pub type ShapeSize = u32;

/// Packs a shape's unpacked value form into a single domain-branded value.
#[must_use]
pub fn pack<D: Domain, T: Shape>(values: <T as Shape>::Values<D>) -> Value<D, T> {
    T::pack_values(values)
}

/// Unpacks a domain-branded value into a shape's value form; for tuple
/// shapes this yields one value per port, ready to destructure.
#[must_use]
pub fn unpack<D: Domain, T: Shape>(value: Value<D, T>) -> <T as Shape>::Values<D> {
    T::make_values(value)
}

/// A hardware expression branded with the clock domain `D` it belongs
/// to. Values of different domains cannot be combined; crossing domains
/// requires an explicit synchronizer.
impl<D: Domain, T: Shape> fmt::Debug for Value<D, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Value")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

pub struct Value<D: Domain, T: Shape> {
    handle: ExprHandle,
    view: T::View<D>,
    _brand: PhantomData<fn() -> (D, T)>,
}

impl<D: Domain, T: Shape> Value<D, T> {
    #[must_use]
    pub(crate) fn from_handle(handle: ExprHandle) -> Self {
        Self {
            handle,
            view: T::make_view(handle),
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub const fn handle(self) -> ExprHandle {
        self.handle
    }

    #[must_use]
    pub const fn view(self) -> T::View<D> {
        self.view
    }
}

impl<D: Domain, T: Shape> Deref for Value<D, T> {
    type Target = T::View<D>;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<D: Domain, T: Shape> DerefMut for Value<D, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
    }
}

impl<D: Domain, T: Shape> Clone for Value<D, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, T: Shape> Copy for Value<D, T> {}

/// Coercion into a value of domain `D` and shape `Out`. Constants (any
/// `Shape` value) coerce into any domain; branded values only coerce
/// into their own; raw integers coerce against the destination shape.
///
/// Sealed: downstream crates cannot implement it (the extension point is
/// [`Shape`]), which keeps the operator impls coherent.
pub trait IntoValue<D: Domain, Out: Shape>: private::Sealed {
    fn into_value(self) -> Value<D, Out>;
}

mod private {
    use crate::capture::Capture;
    use crate::domain::Domain;
    use crate::io::Output;
    use crate::reg::Reg;
    use crate::value::{FieldView, Shape};

    pub trait Sealed {}

    impl<T: Shape> Sealed for T {}
    impl<D: Domain, T: Shape> Sealed for super::Value<D, T> {}
    impl<D: Domain> Sealed for Output<D> {}
    impl<D: Domain, T: Shape> Sealed for Reg<D, T> {}
    impl<D: Domain, T: Shape> Sealed for Capture<D, T> {}
    impl<D: Domain, S: Shape, T: Shape> Sealed for FieldView<D, S, T> {}

    macro_rules! impl_sealed_int {
        ($($ty:ty),* $(,)?) => {
            $(impl Sealed for $ty {})*
        };
    }

    impl_sealed_int!(
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
}

impl<D: Domain, T: Shape> IntoValue<D, T> for T {
    fn into_value(self) -> Value<D, T> {
        elab::literal(&self)
    }
}

impl<D: Domain, T: Shape> IntoValue<D, T> for Value<D, T> {
    fn into_value(self) -> Self {
        self
    }
}

/// Coercion for the value a body completes with: its tail expression or a
/// `return` value.
///
/// Unlike [`IntoValues`], a value with a domain carries it, so the body's
/// output type is checked at elaboration, and unit or shape literals use a
/// placeholder domain - which is what lets a unit-valued tail such as an
/// `if`/`match` statement finish a body without naming a domain.
pub trait ReturnValue {
    type Domain: Domain;
    type Out: Shape;

    fn pack(self) -> Value<Self::Domain, Self::Out>;
}

impl<D: Domain, T: Shape> ReturnValue for Value<D, T> {
    type Domain = D;
    type Out = T;

    fn pack(self) -> Self {
        self
    }
}

impl<D: Domain, T: Shape> ReturnValue for Reg<D, T> {
    type Domain = D;
    type Out = T;

    fn pack(self) -> Value<D, T> {
        self.read()
    }
}

impl<D: Domain, T: Shape> ReturnValue for Capture<D, T> {
    type Domain = D;
    type Out = T;

    fn pack(self) -> Value<D, T> {
        self.read()
    }
}

/// Unit-valued completions have no domain to carry.
impl ReturnValue for () {
    type Domain = LoopDomain;
    type Out = ();

    fn pack(self) -> Value<Self::Domain, ()> {
        elab::literal(&())
    }
}

macro_rules! impl_return_value_literal {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ReturnValue for $ty {
                type Domain = LoopDomain;
                type Out = $ty;

                fn pack(self) -> Value<Self::Domain, $ty> {
                    elab::literal(&self)
                }
            }
        )*
    };
}

impl_return_value_literal!(bool);

impl<const N: usize> ReturnValue for U<N> {
    type Domain = LoopDomain;
    type Out = Self;

    fn pack(self) -> Value<Self::Domain, Self> {
        elab::literal(&self)
    }
}

impl<const N: usize> ReturnValue for Bit<N> {
    type Domain = LoopDomain;
    type Out = Self;

    fn pack(self) -> Value<Self::Domain, Self> {
        elab::literal(&self)
    }
}

macro_rules! impl_return_value_tuple {
    ($($T:ident),*) => {
        impl<D: Domain, $($T: Shape),*> ReturnValue for ($(Value<D, $T>,)*) {
            type Domain = D;
            type Out = ($($T,)*);

            fn pack(self) -> Value<D, Self::Out> {
                let values = <Self as IntoValues<D>>::into_values(self);
                <Self::Out as Shape>::pack_values::<D>(values)
            }
        }

        // A tuple of shape literals, e.g. `(u!(8:1), u!(8:2))`.
        impl<$($T: Shape),*> ReturnValue for ($($T,)*) {
            type Domain = crate::module::LoopDomain;
            type Out = ($($T,)*);

            fn pack(self) -> Value<Self::Domain, Self::Out> {
                let values = <Self as IntoValues<Self::Domain>>::into_values(self);
                <Self::Out as Shape>::pack_values::<Self::Domain>(values)
            }
        }
    };
}

variadics_please::all_tuples!(impl_return_value_tuple, 1, 15, T);

impl<D: Domain, T: Shape + Arith> IntoValue<D, T> for u128 {
    fn into_value(self) -> Value<D, T> {
        T::from_u128(self)
    }
}

macro_rules! impl_into_value_int {
    ($($ty:ty => $to_u128:expr),* $(,)?) => {
        $(
            impl<D: Domain, T: Shape + Arith> IntoValue<D, T> for $ty {
                fn into_value(self) -> Value<D, T> {
                    T::from_u128($to_u128(self))
                }
            }
        )*
    };
}

impl_into_value_int!(
    u8 => u128::from,
    u16 => u128::from,
    u32 => u128::from,
    u64 => u128::from,
    usize => |value: usize| value as u128,
    i8 => |value: i8| u128::from(value.cast_unsigned()),
    i16 => |value: i16| u128::from(value.cast_unsigned()),
    i32 => |value: i32| u128::from(value.cast_unsigned()),
    i64 => |value: i64| u128::from(value.cast_unsigned()),
    i128 => |value: i128| value.cast_unsigned(),
    isize => |value: isize| value.cast_unsigned() as u128,
);

/// Coercion into the argument tuple of a shape, for method calls.
pub trait IntoValues<D: Domain> {
    type Out: Shape<Values<D> = Self::Values>;
    type Values;

    fn into_values(self) -> Self::Values;
}

impl<D: Domain, T: Shape> IntoValues<D> for T {
    type Out = T;
    type Values = <T as Shape>::Values<D>;

    fn into_values(self) -> Self::Values {
        T::make_values(self.into_value())
    }
}

impl<D: Domain, T: Shape> IntoValues<D> for Value<D, T> {
    type Out = T;
    type Values = <T as Shape>::Values<D>;

    fn into_values(self) -> Self::Values {
        T::make_values(self)
    }
}

/// A field of a value's view: either the whole value (`index: None`) or
/// one named field of a struct value.
///
/// Views are pure data - reading a field elaborates a projection, and
/// assigning through `<<=` rewrites the field of the register the view
/// was taken from.
#[derive(Debug)]
pub struct FieldView<D: Domain, S: Shape, T: Shape> {
    parent: ExprHandle,
    index: Option<usize>,
    _brand: PhantomData<fn() -> (D, S, T)>,
}

impl<D: Domain, S: Shape, T: Shape> Clone for FieldView<D, S, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain, S: Shape, T: Shape> Copy for FieldView<D, S, T> {}

impl<D: Domain, S: Shape, T: Shape> FieldView<D, S, T> {
    #[must_use]
    pub fn whole(parent: ExprHandle) -> Self {
        Self {
            parent,
            index: None,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub fn field(parent: ExprHandle, index: usize) -> Self {
        Self {
            parent,
            index: Some(index),
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub fn value(self) -> Value<D, T> {
        self.into_value()
    }

    /// By value: views are [`Copy`], and the read-modify-write targets the
    /// register the view was taken from, so async closures can capture
    /// views without `&mut`.
    pub fn write(self, rhs: impl IntoValue<D, T>) {
        let Some(index) = self.index else {
            panic!("cannot assign to a whole-value view")
        };
        elab::write_struct_field(
            Value::<D, S>::from_handle(self.parent),
            index,
            rhs.into_value(),
        );
    }
}

impl<D: Domain, S: Shape, T: Shape> IntoValue<D, T> for FieldView<D, S, T> {
    fn into_value(self) -> Value<D, T> {
        match self.index {
            None => Value::from_handle(self.parent),
            Some(index) => {
                elab::struct_field::<D, S, T>(Value::<D, S>::from_handle(self.parent), index)
            }
        }
    }
}

impl<D: Domain, S: Shape, T: Shape, Rhs: IntoValue<D, T>> ShlAssign<Rhs> for FieldView<D, S, T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        let Some(index) = self.index else {
            panic!("cannot assign to a whole-value view")
        };
        elab::write_struct_field(
            Value::<D, S>::from_handle(self.parent),
            index,
            rhs.into_value(),
        );
    }
}
