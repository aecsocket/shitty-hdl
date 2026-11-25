pub mod data;
pub mod expr;
pub mod module;

pub mod signal;
pub mod util;
pub mod value;

pub use shitty_hdl_derive::{Shape, module};
pub use typenum;

// LLM slop
mod slop;
pub use slop::{ir, ir::runtime::elaborate, sim, verilog};

pub mod prelude {
    pub use crate::{
        expr::{CmpExt, EqExt, IntoValue, sext, trunc, zext},
        module::{Domain, DomainLabel, In, Out, Sync, comb, if_},
        signal::Signal,
        u,
        value::{Bits, Shape, Width, i, u},
    };
    pub use shitty_hdl_derive::{Shape, module};
    pub use typenum::consts::*;
}

#[doc(hidden)]
pub mod __ {
    pub use crate::{
        ir::runtime::enter_module,
        module::{Module, SignalFlow, if_},
    };
    pub use bon;
}
