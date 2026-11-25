#![doc = include_str!("../README.md")]

pub mod control;
pub mod domain;
mod et;
pub mod port;
pub mod signal;
pub mod value;
pub mod prelude {
    pub use crate::control::if_;
    pub use crate::domain::{Clock, Domain, DomainLabel};
    pub use crate::port::{In, Out};
    pub use crate::signal::Signal;
    pub use crate::value::{IntoValue, Shape, Value};
}

pub use derive::*;
