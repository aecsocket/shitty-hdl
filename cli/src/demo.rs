use front::prelude::*;
use platform::Platform;

#[derive(Domain)]
#[domain(crate = front)]
pub struct Clk;

#[module(crate = front)]
pub fn blinky(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let half_freq = clk.hz.get() / 2;

    let mut led = platform.led.take(0).output(clk);
    let button = platform.button.take(0).input(clk);
    let mut timer = Reg::new(clk, u!(64:0));

    Rule::define_as(|| {
        if timer == half_freq {
            led <<= !led;
            timer <<= u!(0);
        } else if !button {
            timer <<= timer + 1;
        }
    });
}
