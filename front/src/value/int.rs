use crate::{
    ops::{Arith, Bits, Shift, Trunc, Zext},
    value::{Bit, Shape, Value, ValueView},
};

#[derive(Debug, Clone)]
pub struct Uint<const N: usize>(Bit<N>);

impl<const N: usize> Shape for Uint<N> {
    type View = ValueView<Self>;
    type Values = Value<Self>;
}

impl<const N: usize> Bits for Uint<N> {
    const WIDTH: usize = N;
    type Packed = Bit<N>;

    fn pack(this: Value<Self>) -> Value<Self::Packed> {
        todo!();
    }

    fn unpack(bits: Value<Self::Packed>) -> Value<Self> {
        todo!();
    }
}

impl<const N: usize> Arith for Uint<N> {
    fn add(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self> {
        let _ = (lhs, rhs);
        todo!();
    }

    fn sub(lhs: Value<Self>, rhs: Value<Self>) -> Value<Self> {
        let _ = (lhs, rhs);
        todo!();
    }

    fn from_u128(value: u128) -> Value<Self> {
        assert!(
            N >= 128 || (value >> N) == 0,
            "`{value}` cannot fit in `Uint<{N}>`"
        );
        todo!();
    }
}

impl<const N: usize> Shift for Uint<N> {
    fn shl(value: Value<Self>, amount: usize) -> Value<Self> {
        let _ = (value, amount);
        todo!();
    }

    fn shr(value: Value<Self>, amount: usize) -> Value<Self> {
        let _ = (value, amount);
        todo!();
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
}

impl<const N: usize> Trunc for Value<Uint<N>> {
    type To<const TO: usize> = Uint<TO>;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>> {
        todo!();
    }
}

impl<const N: usize> Zext for Value<Uint<N>> {
    type To<const TO: usize> = Uint<TO>;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>> {
        todo!();
    }
}
