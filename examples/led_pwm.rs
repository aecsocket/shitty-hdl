#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

#[derive(Domain)]
pub struct Clk;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

const DUTY_CYCLE: u64 = 3;
const DUTY_CYCLE_PERIOD: u64 = 10;

#[module]
fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let mut timer = Reg::new(clk, uint!(32:0));
    let mut led = platform.led.take(0).output(clk);

    Rule::define_as(|| {
        let led_on = timer < DUTY_CYCLE;
        led <<= led_on;

        if led_on {
            display!("|");
        } else {
            display!(".");
        }

        if timer == DUTY_CYCLE_PERIOD - 1 {
            timer <<= 0;
            displayln!();
        } else {
            timer <<= timer + 1;
        }
    });
}
