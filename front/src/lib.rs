#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod data;
pub mod module;
pub mod ops;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::module::{Interface, Method};
    pub use crate::ops::{
        Arith, ArithCmp, ArithCmpExt, BitCmp, BitCmpExt, Bits, Pack, Repack, Shift, ShlZext,
        ShrTrunc, Trunc, Unpack, Zext,
    };
    pub use crate::reg::Reg;
    pub use crate::value::{Bit, IntoValue, Shape, Uint, Value};
    pub use crate::{bit, sim_assert, uint};
    pub use derive::{Arith, ArithCmp, BitCmp, Bits, Interface, Shape, Shift, hdl, module};
}

pub use derive::*;
