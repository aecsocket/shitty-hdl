mod control;
mod domain;
mod signal;

pub use control::*;
pub use domain::*;
pub use signal::*;

pub trait Module: 'static {
    const NAME: &'static str;
}
