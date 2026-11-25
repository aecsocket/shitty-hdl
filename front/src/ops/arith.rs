use std::ops::{Add, Div, Mul, Rem, Sub};

use crate::{
    air,
    domain::Domain,
    elab,
    ops::Bits,
    reg::Reg,
    value::{IntoValue, Shape, Value},
};

trait ConstToU128 {
    fn const_to_u128(self) -> u128;
}

macro_rules! impl_const_to_u128 {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ConstToU128 for $ty {
                fn const_to_u128(self) -> u128 {
                    u128::from(self)
                }
            }
        )*
    };
}

impl_const_to_u128!(u8, u16, u32, u64, u128);

macro_rules! impl_const_to_u128_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ConstToU128 for $ty {
                fn const_to_u128(self) -> u128 {
                    u128::from(self.cast_unsigned())
                }
            }
        )*
    };
}

impl_const_to_u128_signed!(i8, i16, i32, i64, i128);

pub trait Arith: Bits {
    fn add<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self>;

    fn sub<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self>;

    fn mul<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self>;

    fn from_u128<D: Domain>(value: u128) -> Value<D, Self>;
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Add<Rhs> for Value<D, T> {
    type Output = Self;

    fn add(self, rhs: Rhs) -> Self::Output {
        T::add(self, rhs.into_value())
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Sub<Rhs> for Value<D, T> {
    type Output = Self;

    fn sub(self, rhs: Rhs) -> Self::Output {
        T::sub(self, rhs.into_value())
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Mul<Rhs> for Value<D, T> {
    type Output = Self;

    fn mul(self, rhs: Rhs) -> Self::Output {
        T::mul(self, rhs.into_value())
    }
}

macro_rules! impl_const_arith {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<D: Domain, T: Shape + Arith> Add<$ty> for Value<D, T> {
                type Output = Self;

                fn add(self, rhs: $ty) -> Self::Output {
                    T::add(self, T::from_u128(rhs.const_to_u128()))
                }
            }

            impl<D: Domain, T: Shape + Arith> Sub<$ty> for Value<D, T> {
                type Output = Self;

                fn sub(self, rhs: $ty) -> Self::Output {
                    T::sub(self, T::from_u128(rhs.const_to_u128()))
                }
            }

            impl<D: Domain, T: Shape + Arith> Mul<$ty> for Value<D, T> {
                type Output = Self;

                fn mul(self, rhs: $ty) -> Self::Output {
                    T::mul(self, T::from_u128(rhs.const_to_u128()))
                }
            }

            impl<D: Domain, T: Shape + Arith> Div<$ty> for Value<D, T> {
                type Output = Self;

                fn div(self, rhs: $ty) -> Self::Output {
                    elab::binary(air::BinaryOp::Divide, self, T::from_u128(rhs.const_to_u128()))
                }
            }

            impl<D: Domain, T: Shape + Arith> Rem<$ty> for Value<D, T> {
                type Output = Self;

                fn rem(self, rhs: $ty) -> Self::Output {
                    elab::binary(air::BinaryOp::Remainder, self, T::from_u128(rhs.const_to_u128()))
                }
            }

            impl<D: Domain, T: Shape + Arith> Add<$ty> for Reg<D, T> {
                type Output = Value<D, T>;

                fn add(self, rhs: $ty) -> Self::Output {
                    self.read() + rhs
                }
            }

            impl<D: Domain, T: Shape + Arith> Sub<$ty> for Reg<D, T> {
                type Output = Value<D, T>;

                fn sub(self, rhs: $ty) -> Self::Output {
                    self.read() - rhs
                }
            }

            impl<D: Domain, T: Shape + Arith> Mul<$ty> for Reg<D, T> {
                type Output = Value<D, T>;

                fn mul(self, rhs: $ty) -> Self::Output {
                    self.read() * rhs
                }
            }

            impl<D: Domain, T: Shape + Arith> Div<$ty> for Reg<D, T> {
                type Output = Value<D, T>;

                fn div(self, rhs: $ty) -> Self::Output {
                    self.read() / rhs
                }
            }

            impl<D: Domain, T: Shape + Arith> Rem<$ty> for Reg<D, T> {
                type Output = Value<D, T>;

                fn rem(self, rhs: $ty) -> Self::Output {
                    self.read() % rhs
                }
            }
        )*
    };
}

impl_const_arith!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Add<Rhs> for Reg<D, T> {
    type Output = Value<D, T>;

    fn add(self, rhs: Rhs) -> Self::Output {
        self.read() + rhs
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Sub<Rhs> for Reg<D, T> {
    type Output = Value<D, T>;

    fn sub(self, rhs: Rhs) -> Self::Output {
        self.read() - rhs
    }
}

impl<D: Domain, T: Shape + Arith, Rhs: IntoValue<D, Out = T>> Mul<Rhs> for Reg<D, T> {
    type Output = Value<D, T>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        self.read() * rhs
    }
}
