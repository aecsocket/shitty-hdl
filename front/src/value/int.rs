use crate::{
    domain::Domain,
    elab::{self, Context, ExprHandle},
    ops::{Arith, BitCmp, BitShift, Bits},
    value::{Bit, FieldView, Shape, Value},
};

#[derive(Debug, Default, Clone)]
pub struct U<const N: usize>(pub Bit<N>);

impl<const N: usize> Shape for U<N> {
    type View<D: Domain> = FieldView<D, Self, Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        cx.alloc(air::Type::Uint {
            width: u32::try_from(N).expect("integer width exceeds u32"),
        })
    }

    fn air_value<'dsn>(&self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        cx.bits::<Self>(self.0.words())
    }

    fn make_view<D: Domain>(handle: ExprHandle) -> Self::View<D> {
        FieldView::whole(handle)
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

impl<const N: usize> Bits for U<N> {
    const WIDTH: usize = N;
    type Packed = Bit<N>;

    fn pack<D: Domain>(this: Value<D, Self>) -> Value<D, Self::Packed> {
        elab::cast(air::CastKind::Pack, this)
    }

    fn unpack<D: Domain>(bits: Value<D, Self::Packed>) -> Value<D, Self> {
        elab::cast(air::CastKind::Unpack, bits)
    }
}

impl<const N: usize> BitCmp for U<N> {
    fn gt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::Greater, lhs, rhs)
    }

    fn ge<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::GreaterEqual, lhs, rhs)
    }

    fn lt<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::Less, lhs, rhs)
    }

    fn le<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::LessEqual, lhs, rhs)
    }
}

impl<const N: usize> Arith for U<N> {
    fn add<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self> {
        elab::binary(air::BinaryOp::Add, lhs, rhs)
    }

    fn sub<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self> {
        elab::binary(air::BinaryOp::Subtract, lhs, rhs)
    }

    fn mul<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, Self> {
        elab::binary(air::BinaryOp::Multiply, lhs, rhs)
    }

    fn from_u128<D: Domain>(value: u128) -> Value<D, Self> {
        elab::uint_from_u128(value)
    }
}

impl<const N: usize> BitShift for U<N> {
    fn shl<D: Domain>(value: Value<D, Self>, amount: usize) -> Value<D, Self> {
        elab::shift(air::BinaryOp::ShiftLeft, value, amount)
    }

    fn shr<D: Domain>(value: Value<D, Self>, amount: usize) -> Value<D, Self> {
        elab::shift(air::BinaryOp::ShiftRight, value, amount)
    }
}

impl<const N: usize> U<N> {
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

macro_rules! impl_from_uint {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<const N: usize> From<$ty> for U<N> {
                fn from(value: $ty) -> Self {
                    Self(Bit::from_u128(u128::from(value)))
                }
            }
        )*
    };
}

impl_from_uint!(u8, u16, u32, u64, u128);

impl<const N: usize> From<usize> for U<N> {
    fn from(value: usize) -> Self {
        Self(Bit::from_u128(value as u128))
    }
}

#[macro_export]
macro_rules! u {
    // inferred width
    (0) => {
        $crate::value::U::zero()
    };
    (:1) => {
        $crate::value::U::max()
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        $crate::value::U::constant::<VALUE>()
    }};
    ($e:expr) => {{
        const VALUE: u128 = $e;
        $crate::value::U::constant::<VALUE>()
    }};
    // explicit width
    ($width:literal : 0) => {
        $crate::value::U::<$width>::zero()
    };
    ($width:literal : 1) => {
        $crate::value::U::<$width>::max()
    };
    ($width:literal : $lit:literal) => {{
        const VALUE: u128 = $lit;
        $crate::value::U::<$width>::constant::<VALUE>()
    }};
    ($width:literal : $e:expr) => {{
        const VALUE: u128 = $e;
        $crate::value::U::<$width>::constant::<VALUE>()
    }};
}
