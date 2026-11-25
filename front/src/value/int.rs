use crate::{
    elab::{self, Context, ExprHandle},
    ops::{Arith, ArithCmp, BitCmp, Bits, Shift},
    domain::Domain,
    value::{Bit, Shape, Raw, Value, ValueView},
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

    type Values<D: Domain> = Value<D, Self>;

    fn make_values<D: Domain>(value: Value<D, Self>) -> Self::Values<D> {
        value
    }

    fn pack_values<D: Domain>(values: Self::Values<D>) -> Value<D, Self> {
        values
    }

    fn value_handles<D: Domain>(values: Self::Values<D>) -> Vec<ExprHandle> {
        vec![values.handle()]
    }
}



impl<const N: usize> Bits for Uint<N> {
    const WIDTH: usize = N;
    type Packed = Bit<N>;

    fn pack<D: Domain>(this: Value<D, Self>) -> Value<D, Self::Packed> {
        Value::wrap(elab::cast::<Self, Self::Packed>(
            air::CastKind::Pack,
            this.raw(),
        ))
    }

    fn unpack<D: Domain>(bits: Value<D, Self::Packed>) -> Value<D, Self> {
        Value::wrap(elab::cast::<Self::Packed, Self>(
            air::CastKind::Unpack,
            bits.raw(),
        ))
    }
}

impl<const N: usize> BitCmp for Uint<N> {
    fn eq<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::Equal, lhs.raw(), rhs.raw()))
    }

    fn ne<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::NotEqual, lhs.raw(), rhs.raw()))
    }
}

impl<const N: usize> ArithCmp for Uint<N> {
    fn gt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::Greater, lhs.raw(), rhs.raw()))
    }

    fn ge<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::GreaterEqual, lhs.raw(), rhs.raw()))
    }

    fn lt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::Less, lhs.raw(), rhs.raw()))
    }

    fn le<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        Value::wrap(elab::compare(air::BinaryOp::LessEqual, lhs.raw(), rhs.raw()))
    }
}

impl<const N: usize> Arith for Uint<N> {
    fn add<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self> {
        Value::wrap(elab::binary(air::BinaryOp::Add, lhs.raw(), rhs.raw()))
    }

    fn sub<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self> {
        Value::wrap(elab::binary(air::BinaryOp::Subtract, lhs.raw(), rhs.raw()))
    }

    fn from_u128<D: Domain>(value: u128) -> Value<D, Self> {
        Value::wrap(elab::uint_from_u128::<N>(value))
    }
}

impl<const N: usize> Shift for Uint<N> {
    fn shl(value: Raw<Self>, amount: usize) -> Raw<Self> {
        elab::shift(air::BinaryOp::ShiftLeft, value, amount)
    }

    fn shr(value: Raw<Self>, amount: usize) -> Raw<Self> {
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

