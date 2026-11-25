use std::num::NonZero;

use crate::{
    ActiveOn, Board, Button, Clock, Ddr3, DiffPair, Fpga, Led, Pin, Reset, Resources, Switch,
    Toolchain, Uart,
};

pub const A7_35T: Board = Board {
    fpga: Fpga {
        device: "xc7a35ti",
        ..A7.fpga
    },
    ..A7
};

pub const A7_100T: Board = Board {
    fpga: Fpga {
        device: "xc7a100ti",
        ..A7.fpga
    },
    ..A7
};

const A7: Board = Board {
    toolchain: Toolchain::Vivado {
        design_properties: &[("CFGBVS", "VCCO"), ("CONFIG_VOLTAGE", "3.3")],
    },
    fpga: Fpga {
        device: "",
        package: "csg324",
        speed: "1L",
    },
    resources: Resources {
        clock: &[Clock {
            hz: NonZero::new(100_000_000).unwrap(),
            pin: pin("E3"),
        }],
        default_clock: Some(0),
        reset: &[Reset {
            pin: pin("C2"),
            active_on: ActiveOn::Low,
        }],
        default_reset: Some(0),
        led: &[led("H5"), led("J5"), led("T9"), led("T10")],
        button: &[button("D9"), button("C9"), button("B9"), button("B8")],
        switch: &[switch("A8"), switch("C11"), switch("C10"), switch("A10")],
        uart: &[Uart {
            tx: pin("D10"),
            rx: pin("A9"),
        }],
        ddr3: &[Ddr3 {
            reset_n: ddr_pin("K6"),
            clock: ddr_pair("U9", "V9"),
            clock_enable: ddr_pin("N5"),
            chip_select_n: ddr_pin("U8"),
            write_enable_n: ddr_pin("P5"),
            row_address_strobe_n: ddr_pin("P3"),
            column_address_strobe_n: ddr_pin("M4"),
            address: &[
                ddr_pin("R2"),
                ddr_pin("M6"),
                ddr_pin("N4"),
                ddr_pin("T1"),
                ddr_pin("N6"),
                ddr_pin("R7"),
                ddr_pin("V6"),
                ddr_pin("U7"),
                ddr_pin("R8"),
                ddr_pin("V7"),
                ddr_pin("R6"),
                ddr_pin("U6"),
                ddr_pin("T6"),
                ddr_pin("T8"),
            ],
            bank_address: &[ddr_pin("R1"), ddr_pin("P4"), ddr_pin("P2")],
            data_strobes: &[ddr_pair("N2", "N1"), ddr_pair("U2", "V2")],
            data: &[
                ddr_pin("K5"),
                ddr_pin("L3"),
                ddr_pin("K3"),
                ddr_pin("L6"),
                ddr_pin("M3"),
                ddr_pin("M1"),
                ddr_pin("L4"),
                ddr_pin("M2"),
                ddr_pin("V4"),
                ddr_pin("T5"),
                ddr_pin("U4"),
                ddr_pin("V5"),
                ddr_pin("V1"),
                ddr_pin("T3"),
                ddr_pin("U3"),
                ddr_pin("R3"),
            ],
            data_mask: &[ddr_pin("L1"), ddr_pin("U1")],
            on_die_termination: ddr_pin("R5"),
            attributes: &[("SLEW", "FAST")],
            data_attributes: &[("IN_TERM", "UNTUNED_SPLIT_40")],
        }],
    },
};

const fn led(name: &'static str) -> Led {
    Led {
        pin: pin(name),
        active_on: ActiveOn::High,
    }
}

const fn button(name: &'static str) -> Button {
    Button {
        pin: pin(name),
        active_on: ActiveOn::High,
    }
}

const fn switch(name: &'static str) -> Switch {
    Switch {
        pin: pin(name),
        active_on: ActiveOn::High,
    }
}

const fn pin(name: &'static str) -> Pin {
    Pin {
        name,
        io_standard: IO_STANDARD,
    }
}

const fn ddr_pin(name: &'static str) -> Pin {
    Pin {
        name,
        io_standard: DDR_IO_STANDARD,
    }
}

const fn ddr_pair(p: &'static str, n: &'static str) -> DiffPair {
    DiffPair {
        pos: Pin {
            name: p,
            io_standard: DDR_DIFF_IO_STANDARD,
        },
        neg: Pin {
            name: n,
            io_standard: DDR_DIFF_IO_STANDARD,
        },
    }
}

const IO_STANDARD: &str = "LVCMOS33";
const DDR_IO_STANDARD: &str = "SSTL135";
const DDR_DIFF_IO_STANDARD: &str = "DIFF_SSTL135";
