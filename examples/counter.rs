use shitty_hdl::{ir::Error, prelude::*, sim::Simulator, verilog};

#[derive(Debug, Clone, Shape)]
struct Composite {
    lo: Uint<U32>,
    hi: Uint<U32>,
}

#[module]
pub fn my_counter(
    mut sync: Domain<Sync>,
    start: In<bool>,
    mut count_times_3: Out<Uint<U19>>,
    mut done: Out<bool>,
    mut busy: Out<bool>,
    mut composite: Out<Composite>,
) {
    let mut started = Signal::<bool>::new(false);
    let mut count = Signal::<Uint<U17>>::new(u!(0));
    let mut count_times_2 = Signal::<Uint<U18>>::without_init();

    sync.on(|| done <<= false);
    sync.on(|| busy <<= false);

    comb(|| {
        composite.lo <<= u!((1 << 32) - 1);
        composite.hi <<= u!(0x456);
    });

    if start {
        sync.on(|| started <<= true);
    }

    if started {
        comb(|| {
            count_times_2 <<= zext(count) * u!(2);
            count_times_3 <<= zext(count) * u!(3);
        });

        let new_count = count + u!(1);
        sync.on(|| count <<= new_count);

        if new_count.eq(u!(100_000)) {
            sync.on(|| {
                done <<= true;
                count <<= u!(0);
                started <<= false;
            });
        } else {
            sync.on(|| busy <<= true);
        }
    }
}

fn main() {
    main_().unwrap();
}

fn main_() -> Result<(), Error> {
    let (sim, dut) = Simulator::new(my_counter)?;

    assert!(!sim.get(dut.done)?);
    assert!(!sim.get(dut.busy)?);

    let verilog = verilog::lower(sim.design())?;
    println!("{verilog}");

    Ok(())
}
