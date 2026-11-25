use smallvec::SmallVec;

use crate::{
    ops::{BitCmp, Bits, Trunc, Zext},
    value::{IntoValue, IntoValues, Shape, Value, ValueView},
};

#[derive(Debug, Clone)]
pub struct Bit<const N: usize>(SmallVec<u64, 2>);

#[must_use]
pub const fn bit_cap<const N: usize>() -> usize {
    N.div_ceil(64)
}

impl<const N: usize> Shape for Bit<N> {
    type View = ValueView<Self>;
}

impl<const N: usize> IntoValues for Bit<N> {
    type Out = Self;
    type Values = Value<Self>;

    fn into_values(self) -> Self::Values {
        self.into_value()
    }
}

impl<const N: usize> Bits for Bit<N> {
    const WIDTH: usize = N;
    type Packed = Self;

    fn pack(this: Value<Self>) -> Value<Self> {
        this
    }

    fn unpack(bits: Value<Self>) -> Value<Self> {
        bits
    }
}

impl<const N: usize> BitCmp for Bit<N> {
    fn eq(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        let _ = (lhs, rhs);
        todo!();
    }

    fn ne(lhs: Value<Self>, rhs: Value<Self>) -> Value<bool> {
        let _ = (lhs, rhs);
        todo!();
    }
}

impl<const N: usize> Bit<N> {
    #[must_use]
    pub fn none() -> Self {
        let mut words = SmallVec::new();
        words.resize(bit_cap::<N>(), 0);
        Self(words)
    }

    #[must_use]
    pub fn ones() -> Self {
        let mut words = SmallVec::new();
        words.resize(bit_cap::<N>(), u64::MAX);
        if N % 64 != 0 {
            words[N / 64] = (1_u64 << (N % 64)) - 1;
        }
        Self(words)
    }

    #[must_use]
    pub fn constant<const VALUE: u128>() -> Self {
        assert!(
            N >= 128 || (VALUE >> N) == 0,
            "`VALUE` cannot fit in `Bit<N>`"
        );
        #[expect(clippy::cast_possible_truncation, reason = "intentional")]
        let (lo, hi) = (VALUE as u64, (VALUE >> 64) as u64);
        let Self(mut words) = Self::none();
        if !words.is_empty() {
            words[0] = lo;
        }
        if words.len() > 1 {
            words[1] = hi;
        }
        Self(words)
    }
}

#[macro_export]
macro_rules! bit {
    (0) => {
        $crate::value::Bit::none()
    };
    (:1) => {
        $crate::value::Bit::ones()
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        $crate::value::Bit::constant::<VALUE>()
    }};
}

impl<const N: usize> Trunc for Value<Bit<N>> {
    type To<const TO: usize> = Bit<TO>;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>> {
        todo!();
    }
}

impl<const N: usize> Zext for Value<Bit<N>> {
    type To<const TO: usize> = Bit<TO>;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>> {
        todo!();
    }
}
