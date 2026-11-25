use crate::{
    core::{Bit, bit_cap},
    value::{Shape, Value, ValueView},
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
        const { $crate::core::Uint::ZERO }
    };
    (:1) => {
        const { $crate::core::Uint::MAX }
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        const { $crate::core::Uint::constant::<VALUE>() }
    }};
}
