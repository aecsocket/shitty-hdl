use shitty_hdl::{Bits, prelude::*};

#[derive(Debug, Shape)]
struct Foo {
    lo: u32,
    hi: u32,
}

#[module]
pub fn my_counter(
    clk: Clock,
    start: In<bool>,
    mut count_times_4: Out<u19>,
    mut done: Out<bool>,
    mut busy: Out<bool>,
    mut foo: Out<Foo>,
) {
    let mut started = Signal::<bool>::new(false);
    let mut count = Signal::<u17>::new(0);
    let mut count_times_2 = Signal::<u18>::without_reset();

    clk.on(|| done <<= false);
    clk.on(|| busy <<= false);

    if start {
        clk.on(|| started <<= true);
    }

    if started {
        comb(|| {
            count_times_2 <<= count * 2;
            count_times_4 <<= count * 4;
        });

        let new_count: Value<u17> = count + 1;
        clk.on(|| count <<= new_count);
        if new_count.eq(100_000) {
            clk.on(|| {
                done <<= true;
                count <<= 0;
                started <<= false;
            });
        } else {
            clk.on(|| busy <<= true);
        }
    }
}

fn main() {}
