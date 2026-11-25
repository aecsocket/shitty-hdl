mod demo;
mod task;

pub use demo::blinky;

use anyhow::{Context as _, Result};
use shitty_hdl_board::Board;
use shitty_hdl_core::module::Module;
use shitty_hdl_platform::Platform;
use std::path::{Path, PathBuf};

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

struct Context<Top> {
    hdl_dir: PathBuf,
    board: Board,
    make_top: Box<dyn FnOnce(Platform) -> Top>,
}

pub fn cli<Top: Module>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top + 'static,
) -> Result<()> {
    let dir = manifest_dir.as_ref();
    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(dir)
        .manifest_path(dir.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running cargo metadata")?;
    let hdl_dir = metadata
        .target_directory
        .join("hdl")
        .join(env!("CARGO_CRATE_NAME"));
    let cx = Context {
        hdl_dir: hdl_dir.into_std_path_buf(),
        board,
        make_top: Box::new(move |plat| make_top(plat)),
    };

    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Build => task::build(cx),
        Command::Upload => task::upload(cx),
    }
}

#[macro_export]
macro_rules! cli {
    ($board:expr, $make_top:expr $(,)?) => {
        $crate::cli(env!("CARGO_MANIFEST_DIR"), $board, $make_top)
            .unwrap_or_else(|err| panic!("{err:?}"))
    };
}
