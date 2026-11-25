#![doc = include_str!("../README.md")]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod core;
pub mod module;
pub mod reg;
pub mod value;
pub mod prelude {
    pub use crate::core::Bit;
    pub use crate::module::Action;
    pub use crate::reg::Reg;
    pub use crate::value::{IntoValue, Shape, Value};
    pub use derive::{Shape, module};
}

pub use derive::*;
