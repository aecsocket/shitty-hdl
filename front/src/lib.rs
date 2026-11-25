#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod elements;

pub mod capture;
pub mod domain;
pub mod elab;
pub mod io;
pub mod method;
pub mod module;
pub mod ops;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::capture::Capture;
    pub use crate::domain::{Clock, Domain};
    pub use crate::elab::elaborate;
    pub use crate::elements;
    pub use crate::elements::sim_finish;
    pub use crate::elements::{DisplayArgHardware, DisplayArgText, DisplaySpec, mux};
    pub use crate::io::Output;
    pub use crate::method::Method;
    pub use crate::module::elab;
    pub use crate::module::{Action, Interface, Rule};
    pub use crate::ops::{
        Arith, BitCmp, BitCmpExt, BitEq, BitEqExt, BitShift, Bits, Pack, Repack, ShlZext, ShrTrunc,
        Trunc, Unpack, Zext,
    };
    pub use crate::reg::Reg;
    pub use crate::value::{Bit, IntoValue, IntoValues, Shape, U, Value, Vector};
    pub use crate::value::{pack, unpack};
    pub use crate::{bit, display, displayln, sim_assert, sim_panic, u};

    pub use derive::{Arith, Domain, Interface, Shape, hdl, module};
}

pub use air;
pub use derive::*;

#[doc(hidden)]
pub mod __ {
    pub use crate::elab::context::Context;
    pub use crate::elab::handle::{ActionHandle, ExprHandle, InterfaceHandle};
    pub use crate::elab::if_;
    pub use crate::elab::sim_panic;
    pub use crate::elements::{DisplayArgHardware, DisplayArgText, DisplaySpec};
    pub use bon;
    pub use derive::{display_format, displayln_format};

    use crate::domain::Domain;
    use crate::elab;
    use crate::elab::{
        cast as elab_cast, compare as elab_compare, display as elab_display, module as elab_module,
        subinterface as elab_subinterface,
    };
    use crate::module::Interface;
    use crate::value::{IntoValue, Shape, Value};

    #[must_use]
    pub fn cast<D: Domain, From: Shape, To: Shape>(
        kind: air::CastKind,
        value: Value<D, From>,
    ) -> Value<D, To> {
        elab_cast(kind, value)
    }

    #[must_use]
    pub fn compare<D: Domain, T: Shape>(
        op: air::BinaryOp,
        lhs: Value<D, T>,
        rhs: Value<D, T>,
    ) -> Value<D, bool> {
        elab_compare(op, lhs, rhs)
    }

    pub fn module<I: Interface>(name: &str, definition_path: &str, f: impl FnOnce() -> I) -> I {
        elab_module(name, definition_path, f)
    }

    #[must_use]
    pub fn subinterface(handle: InterfaceHandle, name: &str) -> InterfaceHandle {
        elab_subinterface(handle, name)
    }

    pub fn display_arg<D: Domain, Out: Shape>(value: impl IntoValue<D, Out>) -> ExprHandle {
        value.into_value().handle()
    }

    pub fn display(format: &str, arguments: &[ExprHandle], newline: bool) {
        elab_display(format, arguments, newline);
    }

    pub fn assert<D: Domain>(condition: impl IntoValue<D, bool>, message: &str) {
        elab::assert(condition, message);
    }
}
