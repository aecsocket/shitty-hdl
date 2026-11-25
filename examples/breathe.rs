use board::arty_a7::A7_100T;
use platform::Platform;
use shitty_hdl::prelude::*;

fn main() {
    shitty_hdl::cli!(A7_100T, breathe);
}

#[module(crate = shitty_hdl)]
pub fn breathe(platform: Platform) {
    let clk = platform.default_clock().expect("requires default clock");
    let clock_hz = platform
        .default_clock_hz
        .expect("requires default clock")
        .get();

    let mut led = platform.led.take(0);

    // Fast PWM counter: sweeps 0..255 every 256 clocks.
    let mut pwm = Reg::<Uint<8>>::new(clk, uint!(0));
    // LED duty cycle: the compare threshold against `pwm`.
    let mut duty = Reg::<Uint<8>>::new(clk, uint!(0));
    // Slow tick that advances the duty cycle: one step every half
    // second divided by the 255 duty steps, so each fade takes 1/2 s.
    let mut tick = Reg::<Uint<64>>::new(clk, uint!(0));
    // Direction of the fade: true = brightening, false = dimming.
    let mut rising = Reg::<bool>::new(clk, true);
    // Counts ticks between progress reports.
    let mut report = Reg::<Uint<8>>::new(clk, uint!(0));

    // 255 duty steps per 1/2 second fade.
    let tick_period = clock_hz / 2 / u64::from(u8::MAX);

    Rule::define_as(|| {
        // PWM: the LED is on while the sweep is below the duty cycle.
        pwm <<= pwm + 1;
        led <<= pwm < duty;

        if tick == tick_period {
            tick <<= uint!(0);
            if report == 32u8 {
                report <<= uint!(0);
                display!("duty={duty} rising={rising}");
            } else {
                report <<= report + 1;
            }
            if rising {
                if duty == u8::MAX {
                    display!("peak");
                    rising <<= false;
                    duty <<= duty - 1;
                } else {
                    duty <<= duty + 1;
                }
            } else if duty == uint!(0) {
                display!("trough");
                rising <<= true;
                duty <<= duty + 1;
            } else {
                duty <<= duty - 1;
            }
        } else {
            tick <<= tick + 1;
        }
    });
}
