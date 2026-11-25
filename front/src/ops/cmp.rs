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

pub trait BitCmpOperand<T: BitCmp> {
    fn into_operand(self) -> Value<T>;
}

impl<T: BitCmp, R: IntoValue<Out = T>> BitCmpOperand<T> for R {
    fn into_operand(self) -> Value<T> {
        self.into_value()
    }
}

macro_rules! impl_unsigned_cmp_operand {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<T: BitCmp + Arith> BitCmpOperand<T> for $ty {
                fn into_operand(self) -> Value<T> {
                    T::from_u128(u128::from(self))
                }
            }

            impl<T: Arith> ArithCmpOperand<T> for $ty {
                fn into_operand(self) -> Value<T> {
                    T::from_u128(u128::from(self))
                }
            }
        )*
    };
}

impl_unsigned_cmp_operand!(u8, u16, u32, u64, u128);

pub trait BitCmpExt: IntoValue + Sized
where
    Self::Out: BitCmp,
{
    fn eq(self, rhs: impl BitCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as BitCmp>::eq(self.into_value(), rhs.into_operand())
    }

    fn ne(self, rhs: impl BitCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as BitCmp>::ne(self.into_value(), rhs.into_operand())
    }
}

impl<T: IntoValue> BitCmpExt for T where T::Out: BitCmp {}

pub trait ArithCmpOperand<T: Arith> {
    fn into_operand(self) -> Value<T>;
}

impl<T: Arith, R: IntoValue<Out = T>> ArithCmpOperand<T> for R {
    fn into_operand(self) -> Value<T> {
        self.into_value()
    }
}

pub trait ArithCmpExt: IntoValue + Sized
where
    Self::Out: ArithCmp,
{
    fn gt(self, rhs: impl ArithCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::gt(self.into_value(), rhs.into_operand())
    }

    fn ge(self, rhs: impl ArithCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::ge(self.into_value(), rhs.into_operand())
    }

    fn lt(self, rhs: impl ArithCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::lt(self.into_value(), rhs.into_operand())
    }

    fn le(self, rhs: impl ArithCmpOperand<Self::Out>) -> Value<bool> {
        <Self::Out as ArithCmp>::le(self.into_value(), rhs.into_operand())
    }
}

impl<T: IntoValue> ArithCmpExt for T where T::Out: ArithCmp {}
