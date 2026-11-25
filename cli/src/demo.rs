use front::prelude::*;
use platform::Platform;

#[derive(Debug, Clone, Copy)]
pub struct SysClk;
impl Domain for SysClk {}

#[module(crate = front)]
pub fn blinky(platform: Platform) {
    let default_clock = platform.clock::<SysClk>();
    let half_freq = default_clock.hz.get() / 2;

    let mut led = platform.led.take(0).output(default_clock.clock);
    let button = platform.button.take(0).input(default_clock.clock);
    let mut timer = Reg::new(default_clock.clock, Uint::<64>::zero());

    Rule::define_as(|| {
        if timer == half_freq {
            led <<= !led;
            timer <<= uint!(0);
        } else if !button {
            timer <<= timer + 1;
        }
    });
}
