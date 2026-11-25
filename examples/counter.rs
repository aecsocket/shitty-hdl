#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Shape)]
struct Composite {
    lo: U<32>,
    hi: U<32>,
}

#[module]
pub fn my_counter(
    mut clk: Clock,
    start: In<bool>,
    mut count_times_3: Out<U<19>>,
    mut done: Out<bool>,
    mut busy: Out<bool>,
    mut composite: Out<Composite>,
) {
    let mut started = Signal::<bool>::new(false);
    let mut count = Signal::<U<17>>::new(u!(0));
    let mut count_times_2 = Signal::<U<18>>::without_init();

    clk.on(|| done <<= false);
    clk.on(|| busy <<= false);

    comb(|| {
        composite.lo <<= u!(123);
        composite.hi <<= u!(0x456);
    });

    if start {
        clk.on(|| started <<= true);
    }

    if started {
        comb(|| {
            count_times_2 <<= zext(count) * u!(2);
            count_times_3 <<= zext(count) * u!(3);
        });

        let new_count = count + u!(1);
        clk.on(|| count <<= new_count);

        if new_count.eq(u!(100_000)) {
            clk.on(|| {
                done <<= true;
                count <<= u!(0);
                started <<= false;
            });
        } else {
            clk.on(|| busy <<= true);
        }
    }
}

fn main() {}
