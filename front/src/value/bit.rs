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
        const { $crate::value::Bit::NONE }
    };
    (:1) => {
        const { $crate::value::Bit::ONES }
    };
    ($lit:literal) => {{
        const VALUE: u128 = $lit;
        const { $crate::value::Bit::constant::<VALUE>() }
    }};
}

pub trait Bits: Shape {
    const WIDTH: usize;
    type Packed: Shape;

    #[must_use]
    fn pack(this: Value<Self>) -> Value<Self::Packed>;

    #[must_use]
    fn unpack(bits: Value<Self::Packed>) -> Value<Self>;
}

pub trait Pack {
    type Packed: Shape;

    fn pack(self) -> Value<Self::Packed>;
}

pub trait Unpack {
    type Packed: Shape;

    fn unpack_to<To: Bits<Packed = Self::Packed>>(self) -> Value<To>;
}

impl<T: Bits> Pack for Value<T> {
    type Packed = T::Packed;

    fn pack(self) -> Value<Self::Packed> {
        Bits::pack(self)
    }
}

impl<P: Shape> Unpack for Value<P> {
    type Packed = P;

    fn unpack_to<To: Bits<Packed = P>>(self) -> Value<To> {
        To::unpack(self)
    }
}

pub trait Trunc {
    type To<const TO: usize>: Shape
    where
        [(); bit_cap::<TO>()]:;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:;
}

pub trait Zext {
    type To<const TO: usize>: Shape
    where
        [(); bit_cap::<TO>()]:;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:;
}

impl<const N: usize> Trunc for Value<Bit<N>>
where
    [(); bit_cap::<N>()]:,
{
    type To<const TO: usize>
        = Bit<TO>
    where
        [(); bit_cap::<TO>()]:;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:,
    {
        todo!();
    }
}

impl<const N: usize> Zext for Value<Bit<N>>
where
    [(); bit_cap::<N>()]:,
{
    type To<const TO: usize>
        = Bit<TO>
    where
        [(); bit_cap::<TO>()]:;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>>
    where
        [(); bit_cap::<TO>()]:,
    {
        todo!();
    }
}
