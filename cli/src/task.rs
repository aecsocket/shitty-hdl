use anyhow::{Context as _, Result};
use board::Board;
use platform::Platform;

use crate::Context;

pub fn build(cx: Context) -> Result<()> {
    todo!();
}

pub fn upload(cx: Context) -> Result<()> {
    todo!();
}

struct BuildOutput {}

fn do_build<Top: Module>(
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<BuildOutput> {
    todo!();
}
