use std::{cell::RefCell, fmt, num::NonZero, rc::Rc};

use shitty_hdl_board::{ActiveOn, Board, Pin};
use shitty_hdl_core::{ir, util::Period};

use crate::{ClockBinding, PinBinding, Platform, PlatformElab, ResourceState, Resources, Uart};

impl<T> fmt::Debug for ResourceState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Held { .. } => f.write_str("Held"),
            Self::Used { at } => f.debug_struct("Used").field("at", at).finish(),
        }
    }
}

impl PlatformElab {
    fn reserve(&mut self, pins: &[Pin]) {
        let owner = ir::io::design_id();
        assert!(
            self.owner.is_none_or(|id| id == owner),
            "platform belongs to another elaboration"
        );
        for (index, pin) in pins.iter().enumerate() {
            assert!(
                !pin.name.is_empty() && !pin.io_standard.is_empty(),
                "empty pin or IO standard"
            );
            assert!(
                !self
                    .bindings
                    .iter()
                    .any(|binding| binding.package_pin == pin.name)
                    && !pins[..index].iter().any(|other| other.name == pin.name),
                "package pin {} is assigned more than once",
                pin.name,
            );
        }
        self.owner = Some(owner);
    }

    fn bind(&mut self, signal: ir::SignalId, pin: Pin) {
        self.bindings.push(PinBinding {
            signal,
            bit: 0,
            package_pin: pin.name.into(),
            io_standard: pin.io_standard.into(),
        });
    }
}

fn resources<M: Copy + 'static, T: 'static>(
    metadata: &'static [M],
    elab: &Rc<RefCell<PlatformElab>>,
    allocate: fn(&mut PlatformElab, usize, M) -> T,
) -> Resources<T> {
    Resources {
        items: metadata
            .iter()
            .copied()
            .enumerate()
            .map(|(index, metadata)| {
                let elab = elab.clone();
                ResourceState::Held {
                    make: Box::new(move || allocate(&mut elab.borrow_mut(), index, metadata)),
                }
            })
            .collect(),
    }
}

pub(super) fn from_board(board: Board) -> Platform {
    let default_clock = board.resources.default_clock.map(|index| {
        board
            .resources
            .clock
            .get(index)
            .expect("invalid default clock index")
    });
    let default_clock_hz = default_clock.map(|clock| clock.hz);
    if let Some(index) = board.resources.default_reset {
        assert!(
            index < board.resources.reset.len(),
            "invalid default reset index"
        );
    }
    let elab = Rc::new(RefCell::new(PlatformElab::default()));
    if let Some(metadata) = default_clock {
        let femtos = NonZero::new(1_000_000_000_000_000 / metadata.hz)
            .expect("clock period is below one femtosecond");
        let mut state = elab.borrow_mut();
        state.reserve(&[metadata.pin]);
        let (clock, raw) =
            ir::io::clock(&format!("clock_{}", board.resources.default_clock.unwrap()));
        state.bind(raw, metadata.pin);
        state.clock_bindings.push(ClockBinding {
            signal: raw,
            period: Period::from_femtos(femtos),
        });
        state.default_clock = Some(clock);
    }
    macro_rules! inputs {
        ($field:ident, $name:literal) => {
            resources(board.resources.$field, &elab, |state, index, metadata| {
                state.reserve(&[metadata.pin]);
                let (port, raw) = ir::io::input(
                    &format!("{}_{index}", $name),
                    metadata.active_on == ActiveOn::Low,
                );
                state.bind(raw, metadata.pin);
                port
            })
        };
    }
    let reset = inputs!(reset, "reset");
    let button = inputs!(button, "button");
    let switch = inputs!(switch, "switch");
    let led = resources(board.resources.led, &elab, |state, index, metadata| {
        state.reserve(&[metadata.pin]);
        let (port, raw) =
            ir::io::output(&format!("led_{index}"), metadata.active_on == ActiveOn::Low);
        state.bind(raw, metadata.pin);
        port
    });
    let uart = resources(board.resources.uart, &elab, |state, index, metadata| {
        state.reserve(&[metadata.tx, metadata.rx]);
        let (tx, raw) = ir::io::output(&format!("uart_{index}_tx"), false);
        state.bind(raw, metadata.tx);
        let (rx, raw) = ir::io::input(&format!("uart_{index}_rx"), false);
        state.bind(raw, metadata.rx);
        Uart { tx, rx }
    });
    Platform {
        board,
        default_clock_hz,
        reset,
        led,
        button,
        switch,
        uart,
        ddr3: Resources { items: Vec::new() },
        elab,
    }
}
