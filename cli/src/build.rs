use anyhow::Result;
use cargo_metadata::camino::Utf8PathBuf;
use shitty_hdl_board::Board;
use shitty_hdl_core::{ir::elaborate, module::Module, platform::Platform};

pub fn build<Top: Module>(
    manifest_dir: Utf8PathBuf,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<()> {
    let (design, _) = elaborate::<Top>(|| make_top);
}
