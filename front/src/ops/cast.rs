use crate::{
    domain::Domain,
    elab,
    value::{Bit, U},
    value::{IntoValue, Shape, Value},
};

pub trait Bits: Shape {
    const WIDTH: usize;
    type Packed: Shape;

    #[must_use]
    fn pack<D: Domain>(this: Value<D, Self>) -> Value<D, Self::Packed>;

    #[must_use]
    fn unpack<D: Domain>(bits: Value<D, Self::Packed>) -> Value<D, Self>;
}

pub trait Pack<D: Domain> {
    type Packed: Shape;

    fn pack(self) -> Value<D, Self::Packed>;
}

pub trait Unpack<D: Domain> {
    type Packed: Shape;

    fn unpack<To: Bits<Packed = Self::Packed>>(self) -> Value<D, To>;
}

impl<D: Domain, T: Bits> Pack<D> for Value<D, T> {
    type Packed = T::Packed;

    fn pack(self) -> Value<D, Self::Packed> {
        Bits::pack(self)
    }
}

impl<D: Domain, P: Shape> Unpack<D> for Value<D, P> {
    type Packed = P;

    fn unpack<To: Bits<Packed = P>>(self) -> Value<D, To> {
        To::unpack(self)
    }
}

pub trait Repack<D: Domain>: IntoValue<D> + Sized
where
    Self::Out: Bits,
{
    fn repack<To: Bits<Packed = <Self::Out as Bits>::Packed>>(self) -> Value<D, To> {
        To::unpack(<Self::Out as Bits>::pack(self.into_value()))
    }

    fn repack_trunc<To: Bits>(self) -> Value<D, To>
    where
        <Self::Out as Bits>::Packed: ResizePacked<To::Packed>,
    {
        To::unpack(ResizePacked::trunc_packed(<Self::Out as Bits>::pack(
            self.into_value(),
        )))
    }

    fn repack_zext<To: Bits>(self) -> Value<D, To>
    where
        <Self::Out as Bits>::Packed: ResizePacked<To::Packed>,
    {
        To::unpack(ResizePacked::zext_packed(<Self::Out as Bits>::pack(
            self.into_value(),
        )))
    }
}

impl<D: Domain, T: IntoValue<D>> Repack<D> for T where T::Out: Bits {}

pub trait ResizePacked<To: Shape>: Shape {
    fn trunc_packed<D: Domain>(value: Value<D, Self>) -> Value<D, To>;
    fn zext_packed<D: Domain>(value: Value<D, Self>) -> Value<D, To>;
}

impl<const FROM: usize, const TO: usize> ResizePacked<Bit<TO>> for Bit<FROM> {
    fn trunc_packed<D: Domain>(value: Value<D, Self>) -> Value<D, Bit<TO>> {
        value.trunc::<TO>()
    }

    fn zext_packed<D: Domain>(value: Value<D, Self>) -> Value<D, Bit<TO>> {
        value.zext::<TO>()
    }
}

pub trait Trunc<D: Domain> {
    type To<const TO: usize>: Shape;

    fn trunc<const TO: usize>(self) -> Value<D, Self::To<TO>>;
}

pub trait Zext<D: Domain> {
    type To<const TO: usize>: Shape;

    fn zext<const TO: usize>(self) -> Value<D, Self::To<TO>>;
}

impl<D: Domain, const N: usize> Trunc<D> for Value<D, U<N>> {
    type To<const TO: usize> = U<TO>;

    fn trunc<const TO: usize>(self) -> Value<D, Self::To<TO>> {
        elab::cast(air::CastKind::Truncate, self)
    }
}

impl<D: Domain, const N: usize> Zext<D> for Value<D, U<N>> {
    type To<const TO: usize> = U<TO>;

    fn zext<const TO: usize>(self) -> Value<D, Self::To<TO>> {
        elab::cast(air::CastKind::ZeroExtend, self)
    }
}

impl<D: Domain, const N: usize> Trunc<D> for Value<D, Bit<N>> {
    type To<const TO: usize> = Bit<TO>;

    fn trunc<const TO: usize>(self) -> Value<D, Self::To<TO>> {
        elab::cast(air::CastKind::Truncate, self)
    }
}

impl<D: Domain, const N: usize> Zext<D> for Value<D, Bit<N>> {
    type To<const TO: usize> = Bit<TO>;

    fn zext<const TO: usize>(self) -> Value<D, Self::To<TO>> {
        elab::cast(air::CastKind::ZeroExtend, self)
    }
}
