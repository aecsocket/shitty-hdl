use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Copy, Shape)]
#[shape(width = 3)]
pub enum Tag {
    Wire = 1,
    Graft = 2,
    Erase = 3,
    ExtVal = 4,
    Comb = 5,
    ExtFn = 6,
}

#[derive(Debug, Clone, Shape)]
#[shape(width = 64)]
pub struct Port {
    pub tag: Tag,
    pub payload: Bit<45>,
    pub label: Bit<16>,
}

#[derive(Domain)]
pub struct Clk;

#[module]
pub fn linker(clk: Clock<Clk>) {
    use Tag::*;
    let link = Method::define_as(
        clk,
        async |(a, b): (Value<_, Port>, Value<_, Port>)| match (a.tag, b.tag) {
            (Wire, _) => {}
            (_, Wire) => {}
            sym!(Graft | Erase) => {}
            sym!(ExtVal | Erase) => {}
            sym!(Comb) | sym!(ExtFn) if a.label == b.label => {}
            sym!(Graft, _) | sym!(Comb | ExtFn) => {}
            sym!(Erase, _) | sym!(ExtVal, _) => {}
        },
    );
}
