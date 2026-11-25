use crate::{Bits, Ident, ptr_identity};

ptr_identity! {
    pub struct Type<'d>(TypeDef);
}

#[derive(Debug)]
pub struct TypeDef {
    pub name: Ident,
    pub shape: Shape,
}

pub type ShapeSize = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Bits(ShapeSize),
    Unsigned(ShapeSize),
    Signed(ShapeSize),
}

ptr_identity! {
    pub struct Signal<'d>(SignalDef<'d>);
}

#[derive(Debug)]
pub struct SignalDef<'d> {
    pub name: Ident,
    pub ty: Type<'d>,
    pub init: &'d Bits,
}
