use crate::{
    air,
    domain::Domain,
    elab,
    ops::Arith,
    value::{IntoValue, Shape, Value},
};

pub trait BitEq: Shape {
    fn eq<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn ne<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;
}

/// Every shape is comparable: BSV's `==` is structural, so values of any
/// type compare directly without packing first.
impl<T: Shape> BitEq for T {
    fn eq<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::Equal, lhs, rhs)
    }

    fn ne<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::NotEqual, lhs, rhs)
    }
}

pub trait BitCmp: BitEq + Arith {
    fn gt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn ge<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn lt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn le<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;
}

pub trait BitEqExt<D: Domain, Out>: IntoValue<D, Out> + Sized
where
    Out: BitEq,
{
    fn eq(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitEq>::eq(self.into_value(), rhs.into_value())
    }

    fn ne(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitEq>::ne(self.into_value(), rhs.into_value())
    }
}

impl<D: Domain, T, Out: BitEq> BitEqExt<D, Out> for T where T: IntoValue<D, Out> {}

pub trait BitCmpExt<D: Domain, Out>: IntoValue<D, Out> + Sized
where
    Out: BitCmp,
{
    fn gt(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::gt(self.into_value(), rhs.into_value())
    }

    fn ge(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::ge(self.into_value(), rhs.into_value())
    }

    fn lt(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::lt(self.into_value(), rhs.into_value())
    }

    fn le(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::le(self.into_value(), rhs.into_value())
    }
}

impl<D: Domain, T, Out: BitCmp> BitCmpExt<D, Out> for T where T: IntoValue<D, Out> {}
