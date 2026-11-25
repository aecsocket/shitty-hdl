use front::prelude::*;
use platform::Platform;

// #[module]
pub fn blinky(mut platform: Platform, mut clk: Domain<Clock>) {
    let default_clock_hz = platform.default_clock_hz.expect("requires default clock");
    let half_freq = default_clock_hz.get() / 2;

    let mut led = platform.led.take(0);
    let button = platform.button.take(0);
    let mut timer = Signal::<u64>::new(0);

    if timer.eq(half_freq) {
        clk.on(|| {
            led <<= !led;
            timer <<= 0;
        });
    } else if !button {
        clk.on(|| timer <<= timer + 1);
    }
}
