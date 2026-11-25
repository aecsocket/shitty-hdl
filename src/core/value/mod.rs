mod cmp;
mod num;

pub use {cmp::*, num::*};

use {crate::Bits, std::marker::PhantomData};

pub trait Shape: Clone + Copy {
    type Decomposed;

    //     fn to_bits(self) -> Bits;
    //
    //     fn from_bits(bits: Bits) -> Option<Self>;
}

impl Shape for bool {
    fn to_bits(self) -> Bits {
        Bits::from_word(u64::from(self))
    }

    fn from_bits(bits: Bits) -> Option<Self> {
        match bits.into_words()? {
            [0] => Some(false),
            [1] => Some(true),
            [_] => None,
        }
    }
}

pub struct Value<T: Shape> {
    _phantom: PhantomData<T>,
}

pub trait IntoValue<T: Shape> {
    fn into_value(self) -> Value<T>;
}

impl<T: Shape> IntoValue<T> for T {
    fn into_value(self) -> Value<T> {
        todo!();
    }
}

impl<T: Shape> IntoValue<T> for Value<T> {
    fn into_value(self) -> Value<T> {
        self
    }
}
