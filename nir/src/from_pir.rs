use bumpalo::Bump;
use derive_more::{Display, Error};

use crate as nir;

#[derive(Debug, Display, Error)]
pub enum LowerError {}

#[must_use]
pub fn from_pir<'d>(pir: pir::Module<'d>, bump: &'d Bump) -> Result<nir::Netlist<'d>, LowerError> {
    let top = nir::Module {
        name: pir.name,
        ports: bump.alloc_slice_fill_iter(pir.ports.iter().map(|pir| nir::Port {
            name: pir.name,
            flow: pir.flow,
            value: todo!(),
        })),
        cells: &[],
        children: todo!(),
    };

    Ok(nir::Netlist { top })
}
