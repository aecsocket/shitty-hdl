use front::prelude::*;
use platform::Platform;

#[module(crate = front)]
pub fn blinky(platform: Platform) {
    let clk = platform.default_clock().expect("requires default clock");
    let default_clock_hz = platform.default_clock_hz.expect("requires default clock");
    let half_freq = default_clock_hz.get() / 2;

    let mut led = platform.led.take(0);
    let button = platform.button.take(0);
    let mut timer = Reg::<Uint<64>>::new(clk, uint!(0));

    Rule::define_as(|| {
        if timer == half_freq {
            led <<= !led;
            timer <<= uint!(0);
        } else if !button {
            timer <<= timer + 1;
        }
    });
}
