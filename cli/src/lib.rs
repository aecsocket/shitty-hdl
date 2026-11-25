#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod demo;
mod task;

pub use demo::{Clk, blinky};

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
    /// Simulate the design in Bluesim
    #[command(alias = "sim")]
    Simulate {
        /// Stop the simulation after this many clock cycles
        #[arg(short, long)]
        cycles: Option<u64>,
    },
    /// Build a bitstream for the design
    Build {
        /// Also programs the design onto the currently connected board
        #[arg(long)]
        program: bool,
    },
    /// Program the built design onto the currently connected board
    Program,
}

struct Context {
    out_dir: PathBuf,
    design_key: Option<String>,
    board: Board,
    make_top: Box<dyn FnOnce(Platform)>,
    bsc: Box<dyn BluespecCompiler>,
}

impl Context {
    fn tool_dir(&self, tool: &str) -> PathBuf {
        self.out_dir.join(tool).join(
            self.design_key
                .as_ref()
                .expect("design has not been elaborated"),
        )
    }
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
        .into_std_path_buf();
    let bsc = toolchain_bsc::SystemBluespec::default();
    let cx = Context {
        out_dir,
        design_key: None,
        board,
        make_top: Box::new(|platform| drop(make_top(platform))),
        bsc: Box::new(bsc),
    };

    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Check => task::check(cx),
        Command::Simulate { cycles } => task::simulate(cx, cycles),
        Command::Build { program } => task::build(cx, program),
        Command::Program => task::program(cx),
    }
}
