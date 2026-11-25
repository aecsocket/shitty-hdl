use shitty_hdl::{In, Module, Out, Signal};

pub fn my_counter(mut m: Module, start: In<bool>, count_times_2: Out<u32>, done: Out<bool>) {
    let started = Signal::<bool>::new();
    let count = Signal::<u32>::new();
    let side_count = Signal::<u32>::new().without_reset();

    m.sync += done.set(false);

    m.if_(start, |m| {
        m.sync += started.set(true);
    });

    m.if_(started, |m| {
        m.sync += count.set(count + 1);
        m.sync += side_count.set(count + 1);
        m.comb += count_times_2.set(count * 2);

        m.if_(count.eq(100_000), |m| {
            m.sync += [done.set(true), count.set(0), started.set(false)];
        });
    });
}

fn main() {}
