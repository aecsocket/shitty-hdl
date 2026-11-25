mod demo;
mod task;

pub use demo::blinky;

use anyhow::{Context as _, Result};
use board::Board;
use platform::Platform;
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Instant,
};
use toolchain_bsc::BluespecCompiler;

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Compile the design to Verilog and check diagnostics
    Check,
    /// Build a bitstream for the design
    Build {
        /// Also uploads the design to the currently connected board
        #[arg(long)]
        upload: bool,
    },
    /// Upload the built design to the currently connected board
    Upload,
}

struct Context {
    out_dir: PathBuf,
    board: Board,
    make_top: Box<dyn FnOnce(Platform)>,
    bsc: Box<dyn BluespecCompiler>,
}

#[macro_export]
macro_rules! cli {
    ($board:expr, $make_top:expr $(,)?) => {
        $crate::cli(env!("CARGO_MANIFEST_DIR"), $board, $make_top)
            .unwrap_or_else(|err| panic!("{err:?}"))
    };
}

static START: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn cli<R>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> R + 'static,
) -> Result<()> {
    LazyLock::force(&START);

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("initializing tracing subscriber: {error}"))?;

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
    let bsc = toolchain_bsc::SystemBluespec::default();
    let cx = Context {
        out_dir,
        board,
        make_top: Box::new(|platform| drop(make_top(platform))),
        bsc: Box::new(bsc),
    };

    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Check => task::check(cx),
        Command::Build { upload } => task::build(cx, upload),
        Command::Upload => task::upload(cx),
    }
}
