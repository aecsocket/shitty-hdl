use anyhow::{Context as _, Result};
use board::Board;
use front::elab;
use platform::Platform;

use crate::Context;

pub fn build(cx: Context) -> Result<()> {
    let output = do_build(cx.board, cx.make_top)?;
    println!("{}", output.bsv);

    todo!();
}

pub fn upload(cx: Context) -> Result<()> {
    todo!();
}

struct BuildOutput {
    bsv: String,
}

fn do_build(board: Board, make_top: impl FnOnce(Platform)) -> Result<BuildOutput> {
    let platform = platform::from_board(board);
    let storage = elab::Bump::new();
    let air_module = elab::elaborate(&storage, || make_top(platform));
    let bsv_package = bsv::from_air(&storage, &air_module).context("lowering to BSV")?;

    let mut bsv = String::new();
    bsv::fmt::write(&mut bsv, &bsv_package).context("writing BSV")?;

    Ok(BuildOutput { bsv })
}
