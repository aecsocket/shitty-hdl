use smallvec::SmallVec;

use crate::{
    domain::Domain,
    elab::{self, Context, ExprHandle},
    ops::{BitCmp, Bits},
    value::{Shape, Value, ValueView},
};

#[derive(Debug, Clone)]
pub struct Bit<const N: usize>(SmallVec<u64, 2>);

#[must_use]
pub const fn bit_cap<const N: usize>() -> usize {
    N.div_ceil(64)
}

impl<const N: usize> Shape for Bit<N> {
    type View = ValueView<Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        cx.alloc(air::Type::Bit {
            width: u32::try_from(N).expect("bit width exceeds u32"),
        })
    }

    fn air_value<'dsn>(&self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        cx.bits::<Self>(&self.0)
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

impl<const N: usize> Bits for Bit<N> {
    const WIDTH: usize = N;
    type Packed = Self;

    fn pack<D: Domain>(this: Value<D, Self>) -> Value<D, Self::Packed> {
        this
    }

    fn unpack<D: Domain>(bits: Value<D, Self::Packed>) -> Value<D, Self> {
        bits
    }
}

impl<const N: usize> BitCmp for Bit<N> {
    fn eq<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::Equal, lhs, rhs)
    }

    fn ne<D: Domain>(lhs: Value<D, Self>, rhs: Value<D, Self>) -> Value<D, bool> {
        elab::compare(air::BinaryOp::NotEqual, lhs, rhs)
    }
}

impl<const N: usize> Bit<N> {
    pub(super) fn words(&self) -> &[u64] {
        &self.0
    }

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
        Self::from_u128(VALUE)
    }

    #[must_use]
    pub fn from_u128(value: u128) -> Self {
        assert!(
            N >= 128 || (value >> N) == 0,
            "`value` cannot fit in `Bit<N>`"
        );
        #[expect(clippy::cast_possible_truncation, reason = "intentional")]
        let (lo, hi) = (value as u64, (value >> 64) as u64);
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
