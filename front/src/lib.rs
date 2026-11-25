#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod data;

pub mod domain;
pub mod elab;
pub mod io;
pub mod method;
pub mod module;
pub mod ops;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::domain::{Clock, Domain};
    pub use crate::elab::elaborate;
    pub use crate::method::Method;
    pub use crate::module::{Action, Interface, Rule};
    pub use crate::ops::{
        Arith, ArithCmp, ArithCmpExt, BitCmp, BitCmpExt, Bits, Pack, Repack, Shift, ShlZext,
        ShrTrunc, Trunc, Unpack, Zext,
    };
    pub use crate::reg::Reg;
    pub use crate::value::{Bit, IntoValue, IntoValues, Shape, Uint, Value};
    pub use crate::{bit, display, sim_assert, uint};
    pub use derive::{Arith, ArithCmp, BitCmp, Bits, Interface, Shape, Shift, hdl, module};
}

pub use air;
pub use derive::*;

#[doc(hidden)]
pub mod __ {
    pub use crate::elab::context::Context;
    pub use crate::elab::handle::{ActionHandle, ExprHandle, InterfaceHandle};
    pub use bon;
    pub use derive::display_format;

    pub fn cast<From: crate::value::Shape, To: crate::value::Shape>(
        kind: air::CastKind,
        value: crate::value::Value<From>,
    ) -> crate::value::Value<To> {
        crate::elab::cast(kind, value)
    }

    pub fn compare<T: crate::value::Shape>(
        op: air::BinaryOp,
        lhs: crate::value::Value<T>,
        rhs: crate::value::Value<T>,
    ) -> crate::value::Value<bool> {
        crate::elab::compare(op, lhs, rhs)
    }

    pub fn module<I: crate::module::Interface>(name: &str, f: impl FnOnce() -> I) -> I {
        crate::elab::module(name, f)
    }

    pub fn subinterface(handle: InterfaceHandle, name: &str) -> InterfaceHandle {
        crate::elab::subinterface(handle, name)
    }

    pub fn display_arg(value: impl crate::value::IntoValue) -> ExprHandle {
        value.into_value().handle()
    }

    pub fn display(format: &str, arguments: &[ExprHandle]) {
        crate::elab::display(format, arguments);
    }

    pub fn assert(condition: impl crate::value::IntoValue<Out = bool>, message: &str) {
        crate::elab::assert(condition, message);
    }
}
