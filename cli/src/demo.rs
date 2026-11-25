use front::prelude::*;
use platform::Platform;

#[module(crate = front)]
pub fn blinky(platform: Platform) {
    let default_clock = platform.default_clock();
    let half_freq = default_clock.hz.get() / 2;

    let mut led = platform.led.take(0);
    let button = platform.button.take(0);
    let mut timer = Reg::<Uint<64>>::new(default_clock, uint!(0));

    Rule::define_as(|| {
        if timer == half_freq {
            led <<= !led;
            timer <<= uint!(0);
        } else if !button {
            timer <<= timer + 1;
        }
    });
}
