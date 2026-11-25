use crate::{ActiveOn, Board, Button, Clock, Fpga, Led, Pin, Reset, Resources, Switch, Uart};

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
    fpga: Fpga {
        device: "",
        package: "csg324",
        speed: "1L",
    },
    resources: Resources {
        clock: &[Clock {
            frequency_hz: 100_000_000,
            io_standard: IO_STANDARD,
        }],
        reset: &[Reset {
            pin: pin("C2"),
            active_on: ActiveOn::High,
        }],
        led: &[led("H5"), led("J5"), led("T9"), led("T10")],
        button: &[button("D9"), button("C9"), button("B9"), button("B8")],
        switch: &[switch("A8"), switch("C11"), switch("C10"), switch("A10")],
        uart: &[Uart {
            tx: pin("D10"),
            rx: pin("A9"),
        }],
        ddr3: &[],
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

const IO_STANDARD: &str = "LVCMOS33";
