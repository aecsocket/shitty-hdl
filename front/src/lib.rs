#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod data;

pub mod domain;
pub mod elab;
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
    pub use crate::{bit, sim_assert, uint};
    pub use derive::{Arith, ArithCmp, BitCmp, Bits, Interface, Shape, Shift, hdl, module};
}

pub use air;
pub use derive::*;
