#![doc = include_str!("../README.md")]

use std::ptr;

use derive_more::From;
pub use irt::Ident;

#[derive(Debug, Clone)]
pub struct Design<'d> {
    pub top: Module<'d>,
    pub modules: Vec<Module<'d>>,
}

#[derive(Debug, Clone)]
pub struct Module<'d> {
    pub name: Ident,
    pub ports: Vec<Port<'d>>,
    pub uses_domains: Vec<UseDomain<'d>>,
}

#[derive(Debug, Clone)]
pub struct Port<'d> {
    pub name: Ident,
    pub flow: Flow,
    pub ty: Type<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    In,
    Out,
}

#[derive(Debug, Clone)]
pub struct UseDomain<'d> {
    pub domain: Domain<'d>,
    pub stmts: Vec<Stmt<'d>>,
}

#[derive(Debug, Clone, From)]
pub enum Stmt<'d> {
    Assign(Assign<'d>),
    Switch(Switch<'d>),
}

#[derive(Debug, Clone)]
pub struct Assign<'d> {
    pub into: Target<'d>,
    pub from: Value<'d>,
}

#[derive(Debug, Clone)]
pub struct Switch<'d> {
    // todo
}

#[derive(Debug, Clone, From)]
pub enum Target<'d> {
    Signal(Signal<'d>),
    Slice(Slice<'d>),
}

#[derive(Debug, Clone)]
pub struct Slice<'d> {
    pub into: Signal<'d>,
    pub offset: ShapeSize,
    pub width: ShapeSize,
}

#[derive(Debug, Clone)]
pub struct Value<'d> {
    pub ty: Type<'d>,
    pub kind: ValueKind<'d>,
}

#[derive(Debug, Clone)]
pub enum ValueKind<'d> {
    Const(&'d BitVec),
}

// externally defined values

#[derive(Debug, Clone, Copy)]
pub struct Type<'d>(pub &'d TypeDef);

impl PartialEq for Type<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for Type<'_> {}

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

#[derive(Debug, Clone, From)]
pub enum Domain<'d> {
    Comb,
    Clocked(ClockedDomain<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct ClockedDomain<'d>(pub &'d DomainDef);

impl PartialEq for ClockedDomain<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for ClockedDomain<'_> {}

#[derive(Debug)]
pub struct DomainDef {
    pub name: Ident,
}

#[derive(Debug, Clone, Copy)]
pub struct Signal<'d>(pub &'d SignalDef<'d>);

impl PartialEq for Signal<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for Signal<'_> {}

#[derive(Debug)]
pub struct SignalDef<'d> {
    pub name: Ident,
    pub ty: Type<'d>,
    pub init: &'d BitVec,
}

#[derive(Debug)]
pub struct BitVec(); // todo
