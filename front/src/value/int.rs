use crate::{
    ops::cast::{Bits, Trunc, Zext},
    value::{Bit, Shape, Value, ValueView, bit_cap},
};

#[derive(Debug, Clone, Copy)]
pub struct Uint<const N: usize>(Bit<N>)
where
    [(); bit_cap::<N>()]:;

impl<const N: usize> Shape for Uint<N>
where
    [(); bit_cap::<N>()]:,
{
    type View = ValueView<Self>;
    type Values = Value<Self>;
}

impl<const N: usize> Bits for Uint<N>
where
    [(); bit_cap::<N>()]:,
{
    const WIDTH: usize = N;
    type Packed = Bit<N>;

    fn pack(this: Value<Self>) -> Value<Self::Packed> {
        todo!();
    }

    fn unpack(bits: Value<Self::Packed>) -> Value<Self> {
        todo!();
    }
}

impl<const N: usize> Trunc for Value<Uint<N>>
where
    [(); bit_cap::<N>()]:,
{
    type To<const TO: usize>
        = Uint<TO>
    where
        [(); bit_cap::<TO>()]:;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:,
    {
        todo!();
    }
}

impl<const N: usize> Zext for Value<Uint<N>>
where
    [(); bit_cap::<N>()]:,
{
    type To<const TO: usize>
        = Uint<TO>
    where
        [(); bit_cap::<TO>()]:;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:,
    {
        todo!();
    }
}

impl<const N: usize> Uint<N>
where
    [(); bit_cap::<N>()]:,
{
    pub const ZERO: Self = Self(Bit::NONE);
    pub const MAX: Self = Self(Bit::ONES);

    #[must_use]
    pub const fn constant<const VALUE: u128>() -> Self {
        Self(Bit::constant::<VALUE>())
    }
}

#[macro_export]
macro_rules! uint {
    (0) => {
        const { $crate::value::Uint::ZERO }
    };
    (:1) => {
        const { $crate::value::Uint::MAX }
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        const { $crate::value::Uint::constant::<VALUE>() }
    }};
}
