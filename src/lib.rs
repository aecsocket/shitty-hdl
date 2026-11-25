pub mod data;
pub mod expr;
pub mod module;
pub mod signal;
mod util;
pub mod value;

pub use shitty_hdl_derive::{Shape, module};
pub use typenum;

// LLM slop
mod slop;
pub use slop::ir::runtime::elaborate;
pub use slop::{ir, sim, verilog};

pub mod prelude {
    pub use crate::expr::{CmpExt, EqExt, IntoExpr, sext, trunc, zext};
    pub use crate::module::{Domain, DomainLabel, In, Out, Sync, comb, if_};
    pub use crate::signal::Signal;
    pub use crate::u;
    pub use crate::value::{Bits, Shape, Sint, Uint, Width};
    pub use shitty_hdl_derive::{Shape, module};
    pub use typenum::consts::*;
}

#[doc(hidden)]
pub mod __ {
    pub use crate::ir::runtime::enter_module;
    pub use crate::module::{Module, SignalFlow, if_};
    pub use bon;
}
