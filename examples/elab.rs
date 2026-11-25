#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[derive(Domain)]
pub struct Clk;

const MODE: u32 = 1;
const INVERT: bool = false;

#[module]
fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let mut timer = Reg::new(clk, u!(32:0));
    let mut leds = [
        platform.led.take(0).output(clk),
        platform.led.take(1).output(clk),
        platform.led.take(2).output(clk),
        platform.led.take(3).output(clk),
    ];

    let patterns: [[usize; 4]; 2] = [[0, 1, 2, 3], [3, 2, 1, 0]];

    Rule::define_as(async || {
        timer <<= timer + 1;

        let shifts = match elab(MODE) {
            0 => patterns[0],
            1 => patterns[1],
            _ => [0, 1, 0, 1],
        };

        for i in elab(0..leds.len()) {
            let on = (timer >> shifts[i]) % 2u32 == 1u32;
            if elab(INVERT) {
                leds[i] <<= !on;
            } else {
                leds[i] <<= on;
            }
        }
    });

    Rule::when(timer == 8u32).define_as(|| {
        displayln!("elab example finished at cycle {timer}");
        sim_finish();
    });
}
