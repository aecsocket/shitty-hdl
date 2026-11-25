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
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Lit(Lit),
    Name(Ident),
    #[display("({lhs} {op} {rhs})")]
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
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
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Lit {
    Int(LitInt),
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
