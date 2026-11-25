#![doc = include_str!("../README.md")]

pub mod ast;

mod from_nir;

pub use from_nir::{LowerError, from_nir};
