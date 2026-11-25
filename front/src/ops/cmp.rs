use crate::{
    ops::Arith,
    value::{IntoValue, Shape, Value},
};

pub trait BitCmp: Shape {
    fn eq(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;

    fn ne(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;
}

pub trait ArithCmp: Arith {
    fn gt(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;

    fn ge(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;

    fn lt(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;

    fn le(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool>;
}

pub trait BitCmpExt: IntoValue + Sized
where
    Self::Out: BitCmp,
{
    fn eq(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as BitCmp>::eq(self.into_value(), rhs.into_value())
    }

    fn ne(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as BitCmp>::ne(self.into_value(), rhs.into_value())
    }
}

impl<T: IntoValue> BitCmpExt for T where T::Out: BitCmp {}

pub trait ArithCmpExt: IntoValue + Sized
where
    Self::Out: ArithCmp,
{
    fn gt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::gt(self.into_value(), rhs.into_value())
    }

    fn ge(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::ge(self.into_value(), rhs.into_value())
    }

    fn lt(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::lt(self.into_value(), rhs.into_value())
    }

    fn le(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::le(self.into_value(), rhs.into_value())
    }
}

impl<T: IntoValue> ArithCmpExt for T where T::Out: ArithCmp {}
