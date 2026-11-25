pub use irt::{Bits, Ident, Pattern, ShapeSize, Signal, SignalDef};

use irt::ptr_identity;

#[derive(Debug, Clone, Copy)]
pub struct Netlist<'d> {
    pub top: Module<'d>,
    pub modules: &'d [Module<'d>],
    pub cells: &'d [Cell<'d>],
    pub connections: &'d [Connection<'d>],
    pub signals: &'d [SignalBinding<'d>],
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
    Cell { cell: Cell<'d>, bit: u32 },
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

ptr_identity! {
    pub struct Module<'d>(ModuleDef<'d>);
}

#[derive(Debug)]
pub struct ModuleDef<'d> {
    pub name: Ident,
    pub parent: Option<Module<'d>>,
    pub children: &'d [Module<'d>],
    pub ports: &'d [Port<'d>],
    pub cells: &'d [Cell<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Port<'d> {
    pub name: Ident,
    pub flow: Flow,
    pub value: Value<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    In,
    Out,
}

#[derive(Debug, Clone, Copy)]
pub struct SignalBinding<'d> {
    pub module: Module<'d>,
    pub signal: Signal<'d>,
    pub value: Value<'d>,
}

#[derive(Debug)]
pub struct CellDef<'d> {
    pub module: Module<'d>,
    pub kind: CellKind<'d>,
}

ptr_identity! {
    pub struct Cell<'d>(CellDef<'d>);
}

#[derive(Debug, Clone, Copy)]
pub enum CellKind<'d> {
    Input { width: u32 },
    Unary(Unary<'d>),
    Binary(Binary<'d>),
    Mux(Mux<'d>),
    Match(Match<'d>),
    AssignmentList(AssignmentList<'d>),
    FlipFlop(FlipFlop<'d>),
}

impl CellKind<'_> {
    #[must_use]
    pub const fn output_width(self) -> usize {
        match self {
            Self::Input { width } => width as usize,
            Self::Unary(unary) => match unary.op {
                UnaryOp::Not | UnaryOp::Neg => unary.value.0.len(),
                UnaryOp::Bool | UnaryOp::ReduceAnd | UnaryOp::ReduceOr | UnaryOp::ReduceXor => 1,
            },
            Self::Binary(binary) => match binary.op {
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt(_)
                | BinaryOp::Le(_)
                | BinaryOp::Gt(_)
                | BinaryOp::Ge(_) => 1,
                _ => binary.lhs.0.len(),
            },
            Self::Mux(mux) => mux.when_true.0.len(),
            Self::Match(matcher) => matcher.arms.len(),
            Self::AssignmentList(assignments) => assignments.default.0.len(),
            Self::FlipFlop(flip_flop) => flip_flop.data.0.len(),
        }
    }
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
pub struct AssignmentList<'d> {
    pub default: Value<'d>,
    pub assignments: &'d [Assignment<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Assignment<'d> {
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
