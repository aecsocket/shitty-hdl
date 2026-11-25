use crate::{
    elab::{self, Context},
    ops::{Arith, ArithCmp, BitCmp, Bits, Shift, Trunc, Zext},
    value::{Bit, IntoValue, IntoValues, Shape, Value, ValueView},
};

#[derive(Debug, Clone)]
pub struct Uint<const N: usize>(pub Bit<N>);

impl<const N: usize> Shape for Uint<N> {
    type View = ValueView<Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        cx.alloc(air::Type::Uint {
            width: u32::try_from(N).expect("integer width exceeds u32"),
        })
    }

    fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        cx.bits::<Self>(self.0.words())
    }

    fn make_values(value: Value<Self>) -> Self::Values {
        value
    }

    fn pack_values(values: Self::Values) -> Value<Self> {
        values
    }
}

impl<const N: usize> IntoValues for Uint<N> {
    type Out = Self;
    type Values = Value<Self>;

    fn into_values(self) -> Self::Values {
        self.into_value()
    }
}

impl<const N: usize> Bits for Uint<N> {
    const WIDTH: usize = N;
    type Packed = Bit<N>;

    fn pack(this: Value<Self>) -> Value<Self::Packed> {
        elab::cast::<Self, Self::Packed>(air::CastKind::Pack, this)
    }

    fn unpack(bits: Value<Self::Packed>) -> Value<Self> {
        elab::cast::<Self::Packed, Self>(air::CastKind::Unpack, bits)
    }
}

impl<const N: usize> BitCmp for Uint<N> {
    fn eq(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::Equal, lhs, rhs)
    }

    fn ne(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::NotEqual, lhs, rhs)
    }
}

impl<const N: usize> ArithCmp for Uint<N> {
    fn gt(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::Greater, lhs, rhs)
    }

    fn ge(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::GreaterEqual, lhs, rhs)
    }

    fn lt(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::Less, lhs, rhs)
    }

    fn le(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        elab::compare(air::BinaryOp::LessEqual, lhs, rhs)
    }
}

impl<const N: usize> Arith for Uint<N> {
    fn add(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self> {
        elab::binary(air::BinaryOp::Add, lhs, rhs)
    }

    fn sub(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self> {
        elab::binary(air::BinaryOp::Subtract, lhs, rhs)
    }

    fn from_u128(value: u128) -> Value<Self> {
        elab::uint_from_u128::<N>(value)
    }
}

impl<const N: usize> Shift for Uint<N> {
    fn shl(value: Value<Self>, amount: usize) -> Value<Self> {
        elab::shift(air::BinaryOp::ShiftLeft, value, amount)
    }

    fn shr(value: Value<Self>, amount: usize) -> Value<Self> {
        elab::shift(air::BinaryOp::ShiftRight, value, amount)
    }
}

impl<const N: usize> Uint<N> {
    #[must_use]
    pub fn zero() -> Self {
        Self(Bit::none())
    }

    #[must_use]
    pub fn max() -> Self {
        Self(Bit::ones())
    }

    #[must_use]
    pub fn constant<const VALUE: u128>() -> Self {
        Self(Bit::constant::<VALUE>())
    }
}

#[macro_export]
macro_rules! uint {
    (0) => {
        $crate::value::Uint::zero()
    };
    (:1) => {
        $crate::value::Uint::max()
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        $crate::value::Uint::constant::<VALUE>()
    }};
    ($e:expr) => {{
        const VALUE: u128 = $e;
        $crate::value::Uint::constant::<VALUE>()
    }};
}

impl<const N: usize> Trunc for Value<Uint<N>> {
    type To<const TO: usize> = Uint<TO>;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>> {
        elab::cast::<Uint<N>, Uint<TO>>(air::CastKind::Truncate, self)
    }
}

impl<const N: usize> Zext for Value<Uint<N>> {
    type To<const TO: usize> = Uint<TO>;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>> {
        elab::cast::<Uint<N>, Uint<TO>>(air::CastKind::ZeroExtend, self)
    }
}
