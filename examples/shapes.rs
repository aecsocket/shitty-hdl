#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[derive(Domain)]
pub struct Clk;

#[derive(Debug, Default, Clone, Shape)]
pub struct Rgb {
    pub r: U<8>,
    pub g: U<8>,
    pub b: U<8>,
}

#[module]
pub fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let mut leds = [
        platform.led.take(0).output(clk),
        platform.led.take(1).output(clk),
        platform.led.take(2).output(clk),
    ];
    let rgb = Reg::new(clk, Rgb::default());
    let mut phase = Reg::new(clk, u!(2:0));

    Rule::define_as(|| {
        phase <<= (*phase + 1) % 3;
        let p = phase.read();

        // Snapshot the register as a view; field writes through the view
        // land back in the register.
        let mut rgb = rgb.view();

        // Rotate the channels, one field write per cycle.
        if p == 0 {
            rgb.r <<= rgb.b;
        }
        if p == 1 {
            rgb.g <<= rgb.r;
        }
        if p == 2 {
            rgb.b <<= rgb.g;
        }

        // One LED per channel, on while the channel is nonzero.
        leds[0] <<= rgb.r.value() != 0;
        leds[1] <<= rgb.g.value() != 0;
        leds[2] <<= rgb.b.value() != 0;
    });
}
