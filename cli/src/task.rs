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
    let (design, _top) =
        shitty_hdl_core::ir::elaborate_named(Top::NAME, || make_top(Platform::from_board(board)))
            .context("elaborating design")?;
    let part = format!(
        "{}{}-{}",
        board.fpga.device, board.fpga.package, board.fpga.speed
    );
    let artifacts = {
        let toolchain = match board.toolchain {
            shitty_hdl_board::Toolchain::Vivado { design_properties } => {
                shitty_hdl_core::Toolchain::Vivado {
                    part: &part,
                    design_properties,
                }
            }
            other => anyhow::bail!("unsupported toolchain {other:?}"),
        };
        shitty_hdl_core::lower(&design, toolchain).context("lowering build artifacts")?
    };

    let elab_hash = todo!("hash verilog + constraints");
    if elab_hash == stored_hash {
        return Ok(BuildOutput {});
    }

    todo!("do actual build steps");

    Ok(())
}
