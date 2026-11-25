mod control;
mod domain;
mod signal;

use std::ops::{Deref, DerefMut, ShlAssign};

use crate::{expr::IntoExpr, signal::Signal, value::Shape};
pub use {control::*, domain::*, signal::*};

pub trait Module {}
