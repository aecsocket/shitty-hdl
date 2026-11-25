#![doc = include_str!("../README.md")]

#[cfg(feature = "from_pir")]
mod from_pir;

#[cfg(feature = "from_pir")]
pub use from_pir::{DriverDomain, LowerError, NetLocation, PatternPart, SignalAccess, from_pir};
pub use irt::{Bits, Flow, Ident, Pattern, ShapeSize, Signal, SignalDef};

use derive_more::From;
use irt::ptr_identity;

#[derive(Debug, Clone, Copy)]
pub struct Netlist<'d> {
    pub top: Module<'d>,
    pub connections: &'d [Connection<'d>],
    pub signals: &'d [SignalBinding<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Module<'d> {
    pub name: Ident,
    pub ports: &'d [Port<'d>],
    pub cells: &'d [CellRef<'d>],
    pub children: &'d [Self],
}

#[derive(Debug, Clone, Copy)]
pub struct Port<'d> {
    pub name: Ident,
    pub flow: Flow,
    pub value: Value<'d>,
}

ptr_identity! {
    pub struct CellRef<'d>(Cell<'d>);
}

#[derive(Debug, Clone, Copy, From)]
pub enum Cell<'d> {
    Input(Input),
    Unary(Unary<'d>),
    Binary(Binary<'d>),
    Mux(Mux<'d>),
    Match(Match<'d>),
    AssignmentList(AssignList<'d>),
    FlipFlop(FlipFlop<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct Input {
    pub width: ShapeSize,
}

ptr_identity! {
    pub struct LateNet<'d>(LateNetDef);
}

#[derive(Debug)]
pub struct LateNetDef {
    pub name: Ident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Net<'d> {
    Const(bool),
    Cell { cell: CellRef<'d>, bit: ShapeSize },
    Late(LateNet<'d>),
}

impl Net<'_> {
    pub const ZERO: Self = Self::Const(false);
    pub const ONE: Self = Self::Const(true);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value<'d>(pub &'d [Net<'d>]);

#[derive(Debug, Clone, Copy)]
pub struct Connection<'d> {
    pub from: LateNet<'d>,
    pub to: Net<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct SignalBinding<'d> {
    pub module: Module<'d>,
    pub signal: Signal<'d>,
    pub value: Value<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Unary<'d> {
    pub op: UnaryOp,
    pub value: Value<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
    Bool,
    ReduceAnd,
    ReduceOr,
    ReduceXor,
}

#[derive(Debug, Clone, Copy)]
pub struct Binary<'d> {
    pub op: BinaryOp,
    pub lhs: Value<'d>,
    pub rhs: Value<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signedness {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    FloorDiv(Signedness),
    Mod(Signedness),
    Eq,
    Ne,
    Lt(Signedness),
    Le(Signedness),
    Gt(Signedness),
    Ge(Signedness),
    And,
    Or,
    Xor,
    Shl,
    Shr(Signedness),
}

#[derive(Debug, Clone, Copy)]
pub struct Mux<'d> {
    pub select: Net<'d>,
    pub when_true: Value<'d>,
    pub when_false: Value<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Match<'d> {
    pub enable: Net<'d>,
    pub value: Value<'d>,
    pub arms: &'d [MatchArm<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct MatchArm<'d> {
    pub patterns: &'d [Pattern<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct AssignList<'d> {
    pub default: Value<'d>,
    pub assigns: &'d [Assign<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Assign<'d> {
    pub condition: Net<'d>,
    pub offset: ShapeSize,
    pub value: Value<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct FlipFlop<'d> {
    pub data: Value<'d>,
    pub init: &'d Bits,
    pub clock: Net<'d>,
    pub edge: Edge,
    pub async_reset: Net<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Pos,
    Neg,
}
