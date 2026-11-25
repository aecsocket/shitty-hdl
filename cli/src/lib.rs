mod build;
mod demo;

pub use demo::blinky;
use tracing::debug;

use std::path::Path;

use anyhow::{Context, Result};

use shitty_hdl_board::Board;
use shitty_hdl_core::{module::Module, platform::Platform};

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Build,
    Upload,
}

pub fn cli<Top: Module>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<()> {
    let manifest_dir = manifest_dir.as_ref();
    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(manifest_dir)
        .manifest_path(manifest_dir.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running `cargo metadata`")?;
    let target = metadata.target_directory.join("hdl");
    debug!(?target, "using target directory");

    let platform = todo!();

    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Build => build::build(manifest_dir, board, make_top),
    }
}

#[macro_export]
macro_rules! cli {
    ($board:expr, $make_top:expr $(,)?) => {
        $crate::cli(env!("CARGO_MANIFEST_DIR"), $board, $make_top).expect("command failed")
    };
}
