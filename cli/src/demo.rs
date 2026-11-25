use shitty_hdl_core::{
    platform::{Button, Led, Platform},
    prelude::*,
};

#[module]
pub fn blinky(mut platform: Platform, mut sync: Domain<Sync>) {
    let half_freq = platform.default_clock_period.hertz() / 2;
    let mut led = platform.request(Led);
    let button = platform.request(Button);
    let mut timer = Signal::<u<U64>>::new(u!(0));

    if timer.eq(half_freq) {
        sync.on(|| {
            led <<= !led;
            timer <<= u!(0);
        });
    } else if !button {
        sync.on(|| timer <<= timer + u!(1));
    }
}
