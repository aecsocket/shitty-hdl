use crate::value::{Bit, IntoValue, Shape, Value};

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

    fn unpack<To: Bits<Packed = Self::Packed>>(self) -> Value<To>;
}

impl<T: Bits> Pack for Value<T> {
    type Packed = T::Packed;

    fn pack(self) -> Value<Self::Packed> {
        Bits::pack(self)
    }
}

impl<P: Shape> Unpack for Value<P> {
    type Packed = P;

    fn unpack<To: Bits<Packed = P>>(self) -> Value<To> {
        To::unpack(self)
    }
}

pub trait Repack: IntoValue + Sized
where
    Self::Out: Bits,
{
    fn repack<To: Bits<Packed = <Self::Out as Bits>::Packed>>(self) -> Value<To> {
        To::unpack(<Self::Out as Bits>::pack(self.into_value()))
    }

    fn repack_trunc<To: Bits>(self) -> Value<To>
    where
        <Self::Out as Bits>::Packed: ResizePacked<To::Packed>,
    {
        To::unpack(ResizePacked::trunc_packed(<Self::Out as Bits>::pack(
            self.into_value(),
        )))
    }

    fn repack_zext<To: Bits>(self) -> Value<To>
    where
        <Self::Out as Bits>::Packed: ResizePacked<To::Packed>,
    {
        To::unpack(ResizePacked::zext_packed(<Self::Out as Bits>::pack(
            self.into_value(),
        )))
    }
}

impl<T: IntoValue> Repack for T where T::Out: Bits {}

pub trait ResizePacked<To: Shape>: Shape {
    fn trunc_packed(value: Value<Self>) -> Value<To>;
    fn zext_packed(value: Value<Self>) -> Value<To>;
}

impl<const FROM: usize, const TO: usize> ResizePacked<Bit<TO>> for Bit<FROM> {
    fn trunc_packed(value: Value<Self>) -> Value<Bit<TO>> {
        value.trunc::<TO>()
    }

    fn zext_packed(value: Value<Self>) -> Value<Bit<TO>> {
        value.zext::<TO>()
    }
}

pub trait Trunc {
    type To<const TO: usize>: Shape;

    fn trunc<const TO: usize>(self) -> Value<Self::To<TO>>;
}

pub trait Zext {
    type To<const TO: usize>: Shape;

    fn zext<const TO: usize>(self) -> Value<Self::To<TO>>;
}
