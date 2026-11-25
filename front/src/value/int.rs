use crate::{
    ops::{Bits, Trunc, Zext},
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
