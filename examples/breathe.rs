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
    let mut duty = Reg::new(clk, uint!(32:0));
    let gamma_lut = Vector::<Uint<7>, 100>::from_fn(|i| Uint::from(map_duty(i as u64)));

    Rule::define_as(|| {
        pwm_cycles <<= (pwm_cycles + 1) % DUTY_PERIOD;
        if breathe_cycles == cycles_per_duty_step - 1 {
            breathe_cycles <<= 0;
            duty <<= (duty + 1) % DUTY_PERIOD;
        } else {
            breathe_cycles <<= breathe_cycles + 1;
        }

        let duty_out = gamma_lut.select(duty.read().trunc::<7>());
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
