//! A synthesizable SystemVerilog output subset, not a complete language AST.
//!
//! Widths, constant-expression requirements, driver rules, and module interfaces
//! are the elaborator's responsibility. `Display` preserves expression grouping;
//! `crate::lower` additionally resolves origins and formats the generated text.

use std::num::NonZero;

use crate::{Ident, Origin};
use derive_more::Display;
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ast {
    Module(Module),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    pub origin: Origin,
    pub name: Ident,
    pub params: SmallVec<Param, 4>,
    pub ports: SmallVec<Port, 4>,
    pub items: Vec<Item>,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("parameter {ty} {name} = {value}")]
pub struct Param {
    pub origin: Origin,
    pub ty: Ty,
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("{flow} {ty} {name}")]
pub struct Port {
    pub origin: Origin,
    pub flow: Flow,
    pub ty: Ty,
    pub name: Ident,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    #[display("input")]
    Input,
    #[display("output")]
    Output,
    #[display("inout")]
    Inout,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    #[display("int")]
    IntSigned,
    #[display("int unsigned")]
    IntUnsigned,
    #[display("logic")]
    Logic,
    #[display("logic [{_0}:{_1}]")]
    LogicPacked(Expr, Expr),
    #[display("bit")]
    Bit,
    #[display("bit [{_0}:{_1}]")]
    BitPacked(Expr, Expr),
    #[display("logic signed [{_0}:{_1}]")]
    LogicSignedPacked(Expr, Expr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Lit(Lit),
    Name(Ident),
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Ternary {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
    },
    Index {
        value: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        value: Box<Expr>,
        msb: Box<Expr>,
        lsb: Box<Expr>,
    },
    IndexedSlice {
        value: Box<Expr>,
        base: Box<Expr>,
        width: Box<Expr>,
        direction: SliceDirection,
    },
    Concat(Vec<Expr>),
    Repeat {
        count: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        function: Ident,
        args: Vec<Expr>,
    },
    Clog2(Box<Expr>),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    #[display("+")]
    Add,
    #[display("-")]
    Sub,
    #[display("*")]
    Mul,
    #[display("/")]
    Div,
    #[display("%")]
    Rem,
    #[display("**")]
    Pow,
    #[display("&")]
    BitAnd,
    #[display("|")]
    BitOr,
    #[display("^")]
    BitXor,
    #[display("~^")]
    BitXnor,
    #[display("&&")]
    LogicalAnd,
    #[display("||")]
    LogicalOr,
    #[display("==")]
    Eq,
    #[display("!=")]
    Ne,
    #[display("===")]
    CaseEq,
    #[display("!==")]
    CaseNe,
    #[display("<")]
    Lt,
    #[display("<=")]
    Le,
    #[display(">")]
    Gt,
    #[display(">=")]
    Ge,
    #[display("<<")]
    Shl,
    #[display(">>")]
    Shr,
    #[display("<<<")]
    ArithmeticShl,
    #[display(">>>")]
    ArithmeticShr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lit {
    Int(LitInt),
    Fill(Fill),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LitInt {
    pub origin: Origin,
    pub width: Option<NonZero<u16>>,
    pub radix: Radix,
    pub value: u64,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radix {
    #[display("d")]
    Dec,
    #[display("b")]
    Bin,
    #[display("h")]
    Hex,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Decl(Decl),
    Assign(Assign),
    Process(Process),
    Instance(Instance),
    LocalParam(Param),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decl {
    pub origin: Origin,
    pub kind: DeclKind,
    pub ty: Ty,
    pub name: Ident,
    pub dimensions: Vec<Range>,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclKind {
    Variable,
    Wire,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Range {
    pub msb: Expr,
    pub lsb: Expr,
}

/// An assignment payload shared by continuous and procedural assignments.
///
/// `Display` emits a continuous `assign`; `Stmt::Assign` selects blocking or
/// nonblocking procedural syntax. Continuous assignment origins are emitted as
/// comments because Yosys does not accept an attribute before `assign`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assign {
    pub origin: Origin,
    pub target: Target,
    pub value: Expr,
}

/// Assignment destinations, excluding general arithmetic expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    Name(Ident),
    Index {
        value: Box<Target>,
        index: Expr,
    },
    Slice {
        value: Box<Target>,
        msb: Expr,
        lsb: Expr,
    },
    IndexedSlice {
        value: Box<Target>,
        base: Expr,
        width: Expr,
        direction: SliceDirection,
    },
    Concat(Vec<Target>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SliceDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Process {
    pub origin: Origin,
    pub kind: ProcessKind,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessKind {
    Comb,
    Clocked(Vec<Event>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    pub edge: Edge,
    pub signal: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Edge {
    Pos,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stmt {
    Assign { assign: Assign, kind: AssignKind },
    If(If),
    Block(Vec<Stmt>),
    Case(Case),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignKind {
    Blocking,
    Nonblocking,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct If {
    pub origin: Origin,
    pub condition: Expr,
    pub then_body: Vec<Stmt>,
    pub else_body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case {
    pub origin: Origin,
    pub value: Expr,
    pub arms: Vec<CaseArm>,
    pub default: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaseArm {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Instance {
    pub origin: Origin,
    pub module: Ident,
    pub name: Ident,
    pub params: Vec<NamedArg>,
    pub ports: Vec<Connection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedArg {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Connection {
    pub name: Ident,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fill {
    Zero,
    One,
    X,
    Z,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    #[display("+")]
    Plus,
    #[display("-")]
    Neg,
    #[display("!")]
    LogicalNot,
    #[display("~")]
    BitNot,
    #[display("&")]
    ReduceAnd,
    #[display("~&")]
    ReduceNand,
    #[display("|")]
    ReduceOr,
    #[display("~|")]
    ReduceNor,
    #[display("^")]
    ReduceXor,
    #[display("~^")]
    ReduceXnor,
}
