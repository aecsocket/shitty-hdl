use anyhow::{Context as _, Result};
use shitty_hdl_board::Board;
use shitty_hdl_core::module::Module;
use shitty_hdl_platform::Platform;

use crate::Context;

pub fn build<Top: Module>(cx: Context<Top>) -> Result<()> {
    todo!();
}

pub fn upload<Top: Module>(cx: Context<Top>) -> Result<()> {
    todo!();
}

struct BuildOutput {}

fn do_build<Top: Module>(
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<BuildOutput> {
    let platform = Platform::from_board(board);
    let (design, _) =
        shitty_hdl_core::elaborate::<Top>(|| (make_top)(platform)).context("elaborating design")?;
    let verilog = shitty_hdl_core::verilog::lower(&design).context("lowering to SystemVerilog")?;
    let constraints = todo!("write constraints");

    let elab_hash = todo!("hash verilog + constraints");
    if elab_hash == stored_hash {
        return Ok(BuildOutput {});
    }

    todo!("do actual build steps");

    Ok(())
}
