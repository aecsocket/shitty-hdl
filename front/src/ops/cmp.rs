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

/// Comparison operands: same-domain values and plain unsigned integers
/// (converted to the left-hand side's width).
pub trait CmpOperand<D: Domain, T: BitCmp> {
    fn into_operand(self) -> Value<D, T>;
}

impl<D: Domain, T: BitCmp, R: IntoValue<D, Out = T>> CmpOperand<D, T> for R {
    fn into_operand(self) -> Value<D, T> {
        self.into_value()
    }
}

macro_rules! impl_unsigned_cmp_operand {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<D: Domain, T: BitCmp + crate::ops::Arith> CmpOperand<D, T> for $ty {
                fn into_operand(self) -> Value<D, T> {
                    T::from_u128::<D>(u128::from(self))
                }
            }
        )*
    };
}

impl_unsigned_cmp_operand!(u8, u16, u32, u64, u128);

pub trait BitCmpExt<D: Domain>: IntoValue<D> + Sized
where
    Self::Out: BitCmp,
{
    fn eq(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as BitCmp>::eq(self.into_value(), rhs.into_operand())
    }

    fn ne(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as BitCmp>::ne(self.into_value(), rhs.into_operand())
    }
}

impl<D: Domain, T: IntoValue<D>> BitCmpExt<D> for T where T::Out: BitCmp {}

pub trait ArithCmpExt<D: Domain>: IntoValue<D> + Sized
where
    Self::Out: ArithCmp,
{
    fn gt(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as ArithCmp>::gt(self.into_value(), rhs.into_operand())
    }

    fn ge(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as ArithCmp>::ge(self.into_value(), rhs.into_operand())
    }

    fn lt(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as ArithCmp>::lt(self.into_value(), rhs.into_operand())
    }

    fn le(self, rhs: impl CmpOperand<D, Self::Out>) -> Value<D, bool> {
        <Self::Out as ArithCmp>::le(self.into_value(), rhs.into_operand())
    }
}

impl<D: Domain, T: IntoValue<D>> ArithCmpExt<D> for T where T::Out: ArithCmp {}
