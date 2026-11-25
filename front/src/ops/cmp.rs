use crate::{
    domain::Domain,
    ops::Arith,
    value::{IntoValue, Shape, Value},
};

pub trait BitCmp: Shape {
    fn eq<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn ne<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;
}

pub trait ArithCmp: BitCmp + Arith {
    fn gt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn ge<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn lt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;

    fn le<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool>;
}

pub trait BitCmpExt<D: Domain, Out>: IntoValue<D, Out> + Sized
where
    Out: BitCmp,
{
    fn eq(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::eq(self.into_value(), rhs.into_value())
    }

    fn ne(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as BitCmp>::ne(self.into_value(), rhs.into_value())
    }
}

impl<D: Domain, T, Out: BitCmp> BitCmpExt<D, Out> for T where T: IntoValue<D, Out> {}

pub trait ArithCmpExt<D: Domain, Out>: IntoValue<D, Out> + Sized
where
    Out: ArithCmp,
{
    fn gt(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as ArithCmp>::gt(self.into_value(), rhs.into_value())
    }

    fn ge(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as ArithCmp>::ge(self.into_value(), rhs.into_value())
    }

    fn lt(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as ArithCmp>::lt(self.into_value(), rhs.into_value())
    }

    fn le(self, rhs: impl IntoValue<D, Out>) -> Value<D, bool> {
        <Out as ArithCmp>::le(self.into_value(), rhs.into_value())
    }
}

impl<D: Domain, T, Out: ArithCmp> ArithCmpExt<D, Out> for T where T: IntoValue<D, Out> {}
