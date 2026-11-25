mod control;
mod domain;
mod signal;

pub use {control::*, domain::*, signal::*};

pub trait Module {
    const NAME: &'static str;
}
