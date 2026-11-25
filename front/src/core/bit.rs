use crate::value::{Shape, Value, ValueView};

#[derive(Debug, Clone, Copy)]
pub struct Bit<const N: usize>([u64; bit_cap::<N>()])
where
    [(); bit_cap::<N>()]:;

#[must_use]
pub const fn bit_cap<const N: usize>() -> usize {
    N.div_ceil(64)
}

impl<const N: usize> Shape for Bit<N>
where
    [(); bit_cap::<N>()]:,
{
    type View = ValueView<Self>;
    type Values = Value<Self>;
}

impl<const N: usize> Bit<N>
where
    [(); bit_cap::<N>()]:,
{
    pub const NONE: Self = Self([0; bit_cap::<N>()]);
    pub const ONES: Self = ones();

    #[must_use]
    pub const fn constant<const VALUE: u128>() -> Self {
        assert!(
            N >= 128 || (VALUE >> N) == 0,
            "`VALUE` cannot fit in `Bit<N>`"
        );
        #[expect(clippy::cast_possible_truncation, reason = "intentional")]
        let (lo, hi) = (VALUE as u64, (VALUE >> 64) as u64);

        let mut words = [0; bit_cap::<N>()];
        if !words.is_empty() {
            words[0] = lo;
        }
        if words.len() > 1 {
            words[1] = hi;
        }
        Self(words)
    }
}

const fn ones<const N: usize>() -> Bit<N>
where
    [(); bit_cap::<N>()]:,
{
    let mut words = [u64::MAX; bit_cap::<N>()];
    if N % 64 != 0 {
        words[N / 64] = (1_u64 << (N % 64)) - 1;
    }
    Bit(words)
}

#[macro_export]
macro_rules! bit {
    (0) => {
        const { $crate::core::Bit::NONE }
    };
    (:1) => {
        const { $crate::core::Bit::ONES }
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        const { $crate::core::Bit::constant::<VALUE>() }
    }};
}
