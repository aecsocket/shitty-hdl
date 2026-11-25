use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[module]
fn top(platform: Platform) {
    let clk = platform.default_clock();
}
