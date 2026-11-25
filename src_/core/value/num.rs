use crate::{Bits, IntoValue, Shape, Value};

pub trait ValAdd<Rhs = Self>: Shape {}
pub trait ValSub<Rhs = Self>: Shape {}
pub trait ValMul<Rhs = Self>: Shape {}
pub trait ValDiv<Rhs = Self>: Shape {}

macro_rules! impl_ops_for_ty {
    ($($ty:ty)*) => {
        $(
        impl Shape for $ty {
//             fn to_bits(self) -> Bits {
//                 Bits::from_word(u64::from(self))
//             }
//
//             fn from_bits(bits: Bits) -> Option<Self> {
//                 let [word] = bits.into_words()?;
//
//             }
        }
        impl ValAdd<$ty> for $ty {}
        impl ValSub<$ty> for $ty {}
        impl ValMul<$ty> for $ty {}
        impl ValDiv<$ty> for $ty {}
        )*
    };
}

impl_ops_for_ty!(u8 u16 u32 u64);
impl_ops_for_ty!(i8 i16 i32 i64);

macro_rules! impl_op {
    ($op_ty:ty, $fn:ident, $req:ident, $do:ident) => {
        impl<T: $req<T>, Rhs: IntoValue<T>> $op_ty for Value<T> {
            type Output = $do<T>;

            fn $fn(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        pub struct $do<T: Shape> {
            lhs: Value<T>,
            rhs: Value<T>,
        }

        impl<T: Shape> IntoValue<T> for $do<T> {
            fn into_value(self) -> Value<T> {
                todo!();
            }
        }
    };
}

impl_op!(std::ops::Add<Rhs>, add, ValAdd, DoAdd);
impl_op!(std::ops::Sub<Rhs>, sub, ValSub, DoSub);
impl_op!(std::ops::Mul<Rhs>, mul, ValMul, DoMul);
impl_op!(std::ops::Div<Rhs>, div, ValDiv, DoDiv);

macro_rules! impl_ops {
    ($ty:ty) => {
        impl<T, Rhs> std::ops::Add<Rhs> for $ty
        where
            T: Shape,
            Value<T>: std::ops::Add<Rhs>,
        {
            type Output = <Value<T> as std::ops::Add<Rhs>>::Output;

            fn add(self, rhs: Rhs) -> Self::Output {
                self.into_value() + rhs
            }
        }

        impl<T, Rhs> std::ops::Sub<Rhs> for $ty
        where
            T: Shape,
            Value<T>: std::ops::Sub<Rhs>,
        {
            type Output = <Value<T> as std::ops::Sub<Rhs>>::Output;

            fn sub(self, rhs: Rhs) -> Self::Output {
                self.into_value() - rhs
            }
        }

        impl<T, Rhs> std::ops::Mul<Rhs> for $ty
        where
            T: Shape,
            Value<T>: std::ops::Mul<Rhs>,
        {
            type Output = <Value<T> as std::ops::Mul<Rhs>>::Output;

            fn mul(self, rhs: Rhs) -> Self::Output {
                self.into_value() * rhs
            }
        }

        impl<T, Rhs> std::ops::Div<Rhs> for $ty
        where
            T: Shape,
            Value<T>: std::ops::Div<Rhs>,
        {
            type Output = <Value<T> as std::ops::Div<Rhs>>::Output;

            fn div(self, rhs: Rhs) -> Self::Output {
                self.into_value() / rhs
            }
        }
    };
}
pub(crate) use impl_ops;
