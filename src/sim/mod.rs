use std::cell::RefCell;

use crate::{Module, elaborate::Elaboration};

pub struct Simulator {}

impl Simulator {
    pub fn new<M: Module>(make_dut: impl FnOnce() -> M) {}
}
