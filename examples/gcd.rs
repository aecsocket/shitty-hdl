#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use board::arty_a7::A7_100T;
use platform::Platform;
use shitty_hdl::prelude::*;

#[derive(Domain)]
pub struct Sync;

fn main() {
    shitty_hdl::cli!(A7_100T, gcd);
}

const A: u128 = 1071;
const B: u128 = 462;

/// Euclid's algorithm as a set of guarded atomic rules, the classic
/// Bluespec "hello world" for rule-based design.
///
/// Press button 0 to start computing gcd(1071, 462) = 21.
/// While computing, LED 0 is on. When done, LEDs 1-3 show the low
/// three bits of the result (0b101 = 5).
#[module]
pub fn gcd(platform: Platform) {
    let clk = platform.clock::<Sync>();

    let start = platform.button.take(0).input(clk);
    let mut busy_led = platform.led.take(0).output(clk);
    let mut result_leds = [
        platform.led.take(1).output(clk),
        platform.led.take(2).output(clk),
        platform.led.take(3).output(clk),
    ];

    let mut x = Reg::new(clk, Uint::<32>::zero());
    let mut y = Reg::new(clk, Uint::<32>::zero());
    let mut busy = Reg::new(clk, false);
    let mut done = Reg::new(clk, false);

    // Each rule is an atomic action that fires only when its guard
    // holds; the scheduler picks a consistent set every cycle.

    // Start a new computation on button press.
    Rule::when(!busy & !done & !start).define_as(|| {
        displayln!("starting gcd({A}, {B})");
        x <<= uint!(A);
        y <<= uint!(B);
        busy <<= true;
        done <<= false;
    });

    // Reduce the larger operand; the guards below are mutually
    // exclusive, so the rules never conflict.
    Rule::when(busy & (x >= y) & (y != uint!(0))).define_as(|| {
        displayln!("x={x} y={y}: x -= y -> {}", x - y);
        x <<= x - y;
    });

    Rule::when(busy & (y > x) & (x != uint!(0))).define_as(|| {
        displayln!("x={x} y={y}: y -= x -> {}", y - x);
        y <<= y - x;
    });

    // One operand reached zero: the other holds the GCD.
    Rule::when(busy & (x == uint!(0)) | (y == uint!(0))).define_as(|| {
        let result = x + y;
        displayln!("gcd = {result} (0x{result:x}, 0b{result:b})");
        busy <<= false;
        done <<= true;
    });

    Rule::define_as(|| {
        busy_led <<= !busy;
        let value = x.read();
        result_leds[0] <<= done.read() & (value >> 0 == 1u32);
        result_leds[1] <<= done.read() & (value >> 1 == 1u32);
        result_leds[2] <<= done.read() & (value >> 2 == 1u32);
    });
}
