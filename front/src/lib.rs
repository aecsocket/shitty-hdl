#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod data;
pub mod module;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::module::{Interface, Method};
    pub use crate::reg::Reg;
    pub use crate::value::{
        Bit, Bits, IntoValue, Pack, Repack, Shape, Trunc, Uint, Unpack, Value, Zext,
    };
    pub use crate::{bit, sim_assert, uint};
    pub use derive::{Bits, Interface, Shape, module};
}

pub use derive::*;
