pub mod arty_a7;

#[derive(Debug, Clone, Copy)]
pub struct Board {
    pub fpga: Fpga,
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
    pub reset: &'static [Reset],
    pub led: &'static [Led],
    pub button: &'static [Button],
    pub switch: &'static [Switch],
    pub uart: &'static [Uart],
    pub ddr3: &'static [Ddr3],
}

#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub frequency_hz: u64,
    pub io_standard: &'static str,
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
    // todo
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
