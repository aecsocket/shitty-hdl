use shitty_hdl::{elaborate, prelude::*, sim::Simulator};

#[module]
fn increment(input: In<Uint<U8>>, mut output: Out<Uint<U8>>) {
    comb(|| output <<= input + u!(1));
}

#[module]
fn twice(input: In<Uint<U8>>, mut output: Out<Uint<U8>>) {
    let mut first = increment();
    let mut second = increment();
    comb(|| {
        first.input <<= input;
        second.input <<= first.output;
        output <<= second.output;
    });
}

#[module]
fn register(mut clk: Domain<Sync>, data: In<Uint<U8>>, mut output: Out<Uint<U8>>) {
    clk.on(|| output <<= data);
}

#[module]
fn pipeline(clk: Domain<Sync>, data: In<Uint<U8>>, mut output: Out<Uint<U8>>) {
    let mut first = register();
    let mut second = register();
    comb(|| {
        first.data <<= data;
        second.data <<= first.output;
        output <<= second.output;
    });
}

fn module_name<M: shitty_hdl::module::Module>(_: &M) -> &'static str {
    M::NAME
}

mod nested {
    use shitty_hdl::prelude::*;

    #[module]
    pub fn increment(input: In<Uint<U8>>, mut output: Out<Uint<U8>>) {
        comb(|| output <<= input + u!(1));
    }
}

#[test]
fn module_names_are_full_function_paths() {
    let (design, dut) = elaborate(increment).unwrap();
    assert_eq!(module_name(&dut), concat!(module_path!(), "::increment"));
    assert_eq!(design.name, module_name(&dut));

    let (design, dut) = elaborate(nested::increment).unwrap();
    assert_eq!(
        module_name(&dut),
        concat!(module_path!(), "::nested::increment")
    );
    assert_eq!(design.name, module_name(&dut));

    let (design, dut) = elaborate(|| configured(2)).unwrap();
    assert_eq!(module_name(&dut), concat!(module_path!(), "::configured"));
    assert_eq!(design.name, module_name(&dut));
}

#[test]
fn generic_instantiations_share_the_function_path() {
    let (tagged_design, tagged_dut) = elaborate(|| tagged::<Tagged>()).unwrap();
    let (sync_design, sync_dut) = elaborate(|| tagged::<Sync>()).unwrap();
    assert_eq!(
        module_name(&tagged_dut),
        concat!(module_path!(), "::tagged")
    );
    assert_eq!(module_name(&sync_dut), module_name(&tagged_dut));
    assert_eq!(tagged_design.name, module_name(&tagged_dut));
    assert_eq!(sync_design.name, module_name(&sync_dut));
}

#[test]
fn repeated_instances_are_distinct_and_internal_ports_stay_internal() {
    let (design, dut) = elaborate(twice).unwrap();
    assert_eq!(
        design.signals.iter().filter(|s| s.port.is_some()).count(),
        2
    );
    let names: std::collections::HashSet<_> = design.signals.iter().map(|s| &s.name).collect();
    assert_eq!(names.len(), design.signals.len());
    shitty_hdl::verilog::lower(&design).unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    for input in [0u8, 3, 254, 255] {
        sim.set(dut.input, input).unwrap();
        assert_eq!(
            sim.get(dut.output).unwrap().words(),
            &[input.wrapping_add(2) as u64]
        );
    }
}

#[test]
fn nested_registers_share_the_clock_and_sample_old_state() {
    let (design, dut) = elaborate(pipeline).unwrap();
    assert_eq!(design.domains.len(), 1);
    assert_eq!(
        design.signals.iter().filter(|s| s.port.is_some()).count(),
        2
    );
    shitty_hdl::verilog::lower(&design).unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    sim.set(dut.data, 3u8).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap().words(), &[0]);
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap().words(), &[3]);
    sim.set(dut.data, 9u8).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap().words(), &[3]);
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap().words(), &[9]);
}

struct Tagged;
impl DomainLabel for Tagged {}

#[module]
fn tagged<L: DomainLabel>(mut clock: shitty_hdl::module::Domain<L>, mut output: Out<Uint<U8>>) {
    clock.on(|| output <<= output + u!(1));
}

#[test]
fn generic_label_used_only_by_injected_domain_is_discovered_and_ticks() {
    let (design, dut) = elaborate(|| tagged::<Tagged>()).unwrap();
    assert_eq!(design.domains.len(), 1);
    assert_eq!(
        design.domains[0].id,
        shitty_hdl::ir::DomainId::of::<Tagged>()
    );
    let mut sim = Simulator::from_design(design).unwrap();
    assert_eq!(sim.get(dut.output).unwrap(), u!(0));
    sim.tick_domain::<Tagged>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap(), u!(1));
    sim.tick_domain::<Tagged>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap(), u!(2));
}

#[module]
fn same_label_domains(
    mut first: Domain<Sync>,
    mut second: Domain<Sync>,
    mut output: Out<Uint<U8>>,
) {
    assert_eq!(first.id(), second.id());
    first.on(|| output <<= output + u!(1));
    second.on(|| output <<= output + u!(2));
}

#[test]
fn injected_same_label_domains_share_id_and_preserve_assignment_priority() {
    let (design, dut) = elaborate(same_label_domains).unwrap();
    assert_eq!(design.domains.len(), 1);
    assert_eq!(design.domains[0].id, shitty_hdl::ir::DomainId::of::<Sync>());
    let mut sim = Simulator::from_design(design).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap(), u!(2));
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.output).unwrap(), u!(4));
}

#[module]
fn configured(count: u8, mut clock: Domain<Sync>, mut output: Out<Uint<U8>>) {
    clock.on(|| output <<= Uint::<U8>::from(count));
}

#[module(builder)]
fn configured_builder(count: u8, mut clock: Domain<Sync>, mut output: Out<Uint<U8>>) {
    clock.on(|| output <<= Uint::<U8>::from(count));
}

#[test]
fn ordinary_arguments_and_builder_do_not_require_domain_arguments() {
    let (design, (direct, built)) = shitty_hdl::ir::elaborate("configured_top", || {
        (configured(3), configured_builder().count(7).call())
    })
    .unwrap();
    assert_eq!(design.domains.len(), 1);
    let mut sim = Simulator::from_design(design).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(direct.output).unwrap(), u!(3));
    assert_eq!(sim.get(built.output).unwrap(), u!(7));
}
