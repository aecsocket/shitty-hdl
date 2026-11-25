use shitty_hdl::{In, Out, Signal, comb, module, sync};

#[module]
pub fn my_counter(start: In<bool>, count_times_4: Out<u32>, done: Out<bool>) {
    let started = Signal::<bool>::new(false);
    let count = Signal::<u32>::new(0);
    let count_times_2 = Signal::<u32>::without_reset();

    sync(|| done <<= false);

    if start {
        sync(|| started <<= true);
    }

    if started {
        sync(|| count <<= count + 1);
        comb(|| {
            count_times_2 <<= count * 2;
            count_times_4 <<= count * 4;
        });

        if count == 100_000 {
            sync(|| {
                done <<= true;
                count <<= 0;
                started <<= false;
            });
        }
    }
}

fn main() {}
