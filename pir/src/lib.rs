#![doc = include_str!("../README.md")]

pub use irt::{Bits, Ident, Pattern, Shape, ShapeSize, Signal, SignalDef, Type, TypeDef};

use derive_more::From;
use irt::ptr_identity;

#[derive(Debug, Clone, Copy)]
pub struct Module<'d> {
    pub name: Ident,
    pub ports: &'d [Port<'d>],
    pub signals: &'d [Signal<'d>],
    pub uses_domains: &'d [UseDomain<'d>],
    pub children: &'d [Self],
}

#[derive(Debug, Clone, Copy)]
pub struct Port<'d> {
    pub name: Ident,
    pub flow: Flow,
    pub signal: Signal<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    In,
    Out,
}

#[derive(Debug, Clone, Copy)]
pub struct UseDomain<'d> {
    pub domain: Domain<'d>,
    pub stmts: &'d [Stmt<'d>],
}

#[derive(Debug, Clone, Copy, From)]
pub enum Stmt<'d> {
    Assign(Assign<'d>),
    Switch(Switch<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct Assign<'d> {
    pub into: Target<'d>,
    pub from: Value<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Switch<'d> {
    pub on: Value<'d>,
    pub arms: &'d [Arm<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Arm<'d> {
    pub patterns: &'d [Pattern<'d>],
    pub guard: Option<Value<'d>>,
    pub then: &'d [Stmt<'d>],
}

#[derive(Debug, Clone, Copy, From)]
pub enum Target<'d> {
    Signal(Signal<'d>),
    Slice(Slice<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct Slice<'d> {
    pub into: Signal<'d>,
    pub offset: ShapeSize,
    pub width: ShapeSize,
}

#[derive(Debug, Clone, Copy)]
pub struct Value<'d> {
    pub ty: Type<'d>,
    pub kind: ValueKind<'d>,
}

#[derive(Debug, Clone, Copy, From)]
pub enum ValueKind<'d> {
    Const(&'d Bits),
    Signal(Signal<'d>),
    Unary(Unary<'d>),
    Binary(Binary<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct Unary<'d> {
    pub op: UnaryOp,
    pub value: &'d Value<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Pos,
    Neg,
    Not,
    Bool,
    ReduceAnd,
    ReduceOr,
    ReduceXor,
    AsUnsigned,
    AsSigned,
}

#[derive(Debug, Clone, Copy)]
pub struct Binary<'d> {
    pub op: BinaryOp,
    pub lhs: &'d Value<'d>,
    pub rhs: &'d Value<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

// externally defined values

#[derive(Debug, Clone, Copy, From)]
pub enum Domain<'d> {
    Comb,
    Clocked(ClockedDomain<'d>),
}

ptr_identity! {
    pub struct ClockedDomain<'d>(DomainDef);
}

#[derive(Debug)]
pub struct DomainDef {
    pub name: Ident,
}
