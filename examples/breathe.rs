#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::board::arty_a7::A7_100T;
use shitty_hdl::prelude::*;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[derive(Domain)]
pub struct Clk;

const DUTY_PERIOD: u64 = 100;

#[module]
pub fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let mut led = platform.led.take(0).output(clk);
    let clk_hz = clk.hz.get();
    let cycles_per_duty_step = clk_hz / DUTY_PERIOD;

    let mut pwm_cycles = Reg::new(clk, uint!(32:0));
    let mut breathe_cycles = Reg::new(clk, uint!(32:0));
    let mut duty = Reg::new(clk, uint!(8:0));
    let gamma_lut = Vector::<Uint<7>, 100>::from_fn(|i| Uint::from(map_duty(i as u64)));

    Rule::define_as(|| {
        // duty cycle for LED
        pwm_cycles <<= (pwm_cycles + 1) % DUTY_PERIOD;

        // slower logic that controls the duty
        breathe_cycles <<= (breathe_cycles + 1) % (2 * clk_hz);
        if breathe_cycles % cycles_per_duty_step == 0 {
            duty <<= (duty + 1) % (2 * DUTY_PERIOD);
        }

        // map duty 0..200 to 0..100 triangle shape
        let linear = if duty < DUTY_PERIOD {
            *duty
        } else {
            199 - *duty
        };

        // map gamma
        let duty_out = gamma_lut.select(linear);
        let led_on = pwm_cycles.read() < duty_out.zext::<32>();
        led <<= led_on;
    });
}

#[expect(
    clippy::cast_precision_loss,
    reason = "duty values are far below f64 precision limits"
)]
#[expect(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "gamma is non-negative and bounded by DUTY_PERIOD"
)]
const fn map_duty(n: u64) -> u64 {
    let n = n as f64 / DUTY_PERIOD as f64;
    let gamma = n * n;
    (gamma * DUTY_PERIOD as f64) as u64
}
