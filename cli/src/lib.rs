mod demo;
pub use demo::blinky;

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

pub fn cli<Top: Module>(board: Board, make_top: impl FnOnce(Platform) -> Top) {
    let target = std::env::var("CARGO_TARGET_DIR").unwrap();
    println!("target: {target:?}");

    let cli = <Cli as clap::Parser>::parse();
    // match cli.command {
    //     Command::Build => {}
    // }
}
