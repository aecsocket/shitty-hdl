use shitty_hdl::{prelude::*, u17, u18, u19};

// struct Foo {
//     lo: u32,
//     hi: u32,
// }
//
// struct FooSignals {
//     lo: Signal<u32>,
//     hi: Signal<u32>,
// }
//
// impl Shape for Foo {
//     type Signals = FooSignals;
// }
//
#[module]
pub fn my_counter(
    clk: Clock,
    start: In<bool>,
    mut count_times_4: Out<u19>,
    mut done: Out<bool>,
    mut busy: Out<bool>,
) {
    let mut started = Signal::<bool>::new(false);
    let mut count = Signal::<u17>::new(u17!(0));
    let mut count_times_2 = Signal::<u18>::without_init();

    clk.on(|| done <<= false);
    clk.on(|| busy <<= false);

    if start {
        clk.on(|| started <<= true);
    }

    if started {
        comb(|| {
            count_times_2 <<= zext::<_, _, 19>(count) * u19!(2);
            count_times_4 <<= zext(count) * u19!(4);
        });

        let new_count = count + 1;
        clk.on(|| count <<= new_count);
        if new_count.eq(100_000) {
            clk.on(|| {
                done <<= true;
                count <<= u17!(0);
                started <<= false;
            });
        } else {
            clk.on(|| busy <<= true);
        }
    }
}

fn main() {}
