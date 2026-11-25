use front::prelude::*;
use platform::Platform;

#[derive(Domain)]
#[domain(crate = front)]
pub struct Sync;

#[module(crate = front)]
pub fn blinky(platform: Platform) {
    let clk = platform.clock::<Sync>();
    let half_freq = clk.hz.get() / 2;

    let mut led = platform.led.take(0).output(clk.clock);
    let button = platform.button.take(0).input(clk.clock);
    let mut timer = Reg::new(clk.clock, Uint::<64>::zero());

    Rule::define_as(|| {
        if timer == half_freq {
            led <<= !led;
            timer <<= uint!(0);
        } else if !button {
            timer <<= timer + 1;
        }
    });
}
