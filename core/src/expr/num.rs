use std::ops::{Add, Mul, Sub};

use crate::{
    expr::{IntoValue, Value},
    ir::BinaryOp,
    module::{In, Out},
    signal::{Signal, SignalView},
    value::{Shape, Width, i, u},
};

pub trait LowerAdd {}
impl<N: Width> LowerAdd for u<N> {}
impl<N: Width> LowerAdd for i<N> {}
pub trait LowerSub {}
impl<N: Width> LowerSub for u<N> {}
impl<N: Width> LowerSub for i<N> {}
pub trait LowerMul {}
impl<N: Width> LowerMul for u<N> {}
impl<N: Width> LowerMul for i<N> {}

macro_rules! impl_for {
    ($($ty:ident)*) => {$(
        impl<T: Shape + LowerAdd, Rhs: IntoValue<Out = T>> Add<Rhs> for $ty<T> {
            type Output = Value<T>;
            #[track_caller]
            fn add(self, rhs: Rhs) -> Value<T> { Value { imp: self.into_value().imp.binary(rhs.into_value().imp, BinaryOp::Add) } }
        }
        impl<T: Shape + LowerSub, Rhs: IntoValue<Out = T>> Sub<Rhs> for $ty<T> {
            type Output = Value<T>;
            #[track_caller]
            fn sub(self, rhs: Rhs) -> Value<T> { Value { imp: self.into_value().imp.binary(rhs.into_value().imp, BinaryOp::Sub) } }
        }
        impl<T: Shape + LowerMul, Rhs: IntoValue<Out = T>> Mul<Rhs> for $ty<T> {
            type Output = Value<T>;
            #[track_caller]
            fn mul(self, rhs: Rhs) -> Value<T> { Value { imp: self.into_value().imp.binary(rhs.into_value().imp, BinaryOp::Mul) } }
        }
    )*};
}
impl_for!(Value Signal SignalView In Out);

pub trait CmpExt: IntoValue {
    fn gt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
    fn lt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
    fn ge(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
    fn le(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
}

impl<E: IntoValue> CmpExt for E
where
    E::Out: LowerAdd,
{
    #[track_caller]
    fn gt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Gt),
        }
    }
    #[track_caller]
    fn lt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Lt),
        }
    }
    #[track_caller]
    fn ge(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Ge),
        }
    }
    #[track_caller]
    fn le(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Le),
        }
    }
}
