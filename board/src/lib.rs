use std::num::NonZero;

pub mod arty_a7;

#[derive(Debug, Clone, Copy)]
pub struct Board {
    pub fpga: Fpga,
    pub toolchain: Toolchain,
    pub resources: Resources,
}

#[derive(Debug, Clone, Copy)]
pub struct Fpga {
    pub package: &'static str,
    pub speed: &'static str,
    pub device: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Resources {
    pub clock: &'static [Clock],
    pub default_clock: Option<usize>,
    pub reset: &'static [Reset],
    pub default_reset: Option<usize>,
    pub led: &'static [Led],
    pub button: &'static [Button],
    pub switch: &'static [Switch],
    pub uart: &'static [Uart],
    pub ddr3: &'static [Ddr3],
}

#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub hz: NonZero<u64>,
    pub pin: Pin,
}

#[derive(Debug, Clone, Copy)]
pub struct Reset {
    pub pin: Pin,
    pub active_on: ActiveOn,
}

#[derive(Debug, Clone, Copy)]
pub struct Led {
    pub pin: Pin,
    pub active_on: ActiveOn,
}

#[derive(Debug, Clone, Copy)]
pub struct Button {
    pub pin: Pin,
    pub active_on: ActiveOn,
}

#[derive(Debug, Clone, Copy)]
pub struct Switch {
    pub pin: Pin,
    pub active_on: ActiveOn,
}

#[derive(Debug, Clone, Copy)]
pub struct Uart {
    pub tx: Pin,
    pub rx: Pin,
}

#[derive(Debug, Clone, Copy)]
pub struct Ddr3 {
    pub reset_n: Pin,
    pub clock: DiffPair,
    pub clock_enable: Pin,
    pub chip_select_n: Pin,
    pub write_enable_n: Pin,
    pub row_address_strobe_n: Pin,
    pub column_address_strobe_n: Pin,
    pub address: &'static [Pin],
    pub bank_address: &'static [Pin],
    pub data_strobes: &'static [DiffPair],
    pub data: &'static [Pin],
    pub data_mask: &'static [Pin],
    pub on_die_termination: Pin,
    pub attributes: Attributes,
    pub data_attributes: Attributes,
}

#[derive(Debug, Clone, Copy)]
pub struct DiffPair {
    pub pos: Pin,
    pub neg: Pin,
}

#[derive(Debug, Clone, Copy)]
pub struct Pin {
    pub name: &'static str,
    pub io_standard: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActiveOn {
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toolchain {
    Vivado,
    Ise,
    Quartus,
    IceStorm,
    Trellis,
}

pub type Attributes = &'static [(&'static str, &'static str)];
