mod demo;
mod task;

pub use demo::blinky;

use anyhow::{Context as _, Result};
use board::Board;
use platform::Platform;
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

struct Context {
    out_dir: PathBuf,
    board: Board,
    make_top: Box<dyn FnOnce(Platform)>,
}

#[macro_export]
macro_rules! cli {
    ($board:expr, $make_top:expr $(,)?) => {
        $crate::cli(env!("CARGO_MANIFEST_DIR"), $board, $make_top)
            .unwrap_or_else(|err| panic!("{err:?}"))
    };
}

pub fn cli<R>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> R + 'static,
) -> Result<()> {
    let dir = manifest_dir.as_ref();
    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(dir)
        .manifest_path(dir.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running cargo metadata")?;
    let out_dir = metadata
        .target_directory
        .join("hdl")
        .join(env!("CARGO_CRATE_NAME"))
        .into_std_path_buf();
    let cx = Context {
        out_dir,
        board,
        make_top: Box::new(|platform| drop(make_top(platform))),
    };

    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Build => task::build(cx),
        Command::Upload => task::upload(cx),
    }
}
