use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

#[derive(Domain)]
pub struct Sync;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[module]
fn top(platform: Platform) {
    let _clk = platform.clock::<Sync>();
}
