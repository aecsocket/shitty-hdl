use derive_more::{Display, Error};

use crate as nir;

#[derive(Debug, Display, Error)]
pub enum LowerError {}

#[must_use]
pub fn from_pir<'d>(pir: &'d pir::Module) -> Result<nir::Netlist<'d>, LowerError> {
    todo!();
}
