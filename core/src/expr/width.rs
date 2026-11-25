use typenum::{B1, IsGreaterOrEqual, IsLessOrEqual};

use crate::{
    expr::{IntoValue, Value},
    value::{Shape, Signed, Unsigned, Width, i, u},
};

// zext

pub trait CanZext<To>: Unsigned {}
impl<From: Width, To: Width> CanZext<u<To>> for u<From> where To: IsGreaterOrEqual<From, Output = B1>
{}

#[track_caller]
pub fn zext<To: Shape>(lhs: impl IntoValue<Out: CanZext<To>>) -> Value<To> {
    Value {
        imp: lhs.into_value().imp.resize::<To>(true),
    }
}

// sext

pub trait CanSext<To>: Signed {}
impl<From: Width, To: Width> CanSext<i<To>> for i<From> where To: IsGreaterOrEqual<From, Output = B1>
{}

#[track_caller]
pub fn sext<To: Shape>(lhs: impl IntoValue<Out: CanSext<To>>) -> Value<To> {
    Value {
        imp: lhs.into_value().imp.resize::<To>(true),
    }
}

// trunc

pub trait CanTrunc<To>: Shape {
    type Truncated;
}

impl<From: Width, To: Width> CanTrunc<u<To>> for u<From>
where
    To: IsLessOrEqual<From, Output = B1>,
{
    type Truncated = u<To>;
}

impl<From: Width, To: Width> CanTrunc<i<To>> for i<From>
where
    To: IsLessOrEqual<From, Output = B1>,
{
    type Truncated = i<To>;
}

#[track_caller]
pub fn trunc<To: Shape>(lhs: impl IntoValue<Out: CanTrunc<To>>) -> Value<To> {
    Value {
        imp: lhs.into_value().imp.resize::<To>(false),
    }
}
