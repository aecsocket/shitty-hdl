use platform::Platform;

#[module]
pub fn blinky(mut platform: Platform, mut sync: Domain<Sync>) {
    let default_clock_hz = platform.default_clock_hz.expect("requires default clock");
    let half_freq = default_clock_hz.get() / 2;
    let mut led = platform.led.take(0);
    let button = platform.button.take(0);
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
