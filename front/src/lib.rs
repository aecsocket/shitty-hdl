#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod core;
pub mod module;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::core::Bit;
    pub use crate::module::{Interface, Method};
    pub use crate::reg::Reg;
    pub use crate::value::{IntoValue, Shape, Value};
    pub use crate::{bit, sim_assert, uint};
    pub use derive::{Interface, Shape, module};
}

pub use derive::*;
