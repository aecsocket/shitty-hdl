use std::{cell::RefCell, collections::HashSet, num::NonZero, rc::Rc};

use crate::*;
use anyhow::{Result, anyhow, ensure};

#[derive(Debug, Default)]
pub(super) struct ResourceState {
    owner: Option<ir::DesignId>,
    requested: HashSet<(&'static str, usize)>,
    pins: HashSet<&'static str>,
    pub(super) bindings: Vec<PinBinding>,
    pub(super) clocks: Vec<ClockBinding>,
}

#[derive(Debug, Clone)]
pub(super) struct Collection<T: 'static> {
    metadata: &'static [T],
    pub(super) state: Rc<RefCell<ResourceState>>,
}

fn period(clock: &board::Clock) -> Result<Period> {
    let femtos = 1_000_000_000_000_000 / clock.hz;
    let femtos =
        NonZero::new(femtos).ok_or_else(|| anyhow!("clock period is below one femtosecond"))?;
    Ok(Period::from_femtos(femtos))
}

pub(super) fn from_board(board: Board) -> Result<Platform> {
    let default_clock_period = period(
        board
            .resources
            .clock
            .first()
            .ok_or_else(|| anyhow!("board has no default clock"))?,
    )?;
    for clock in board.resources.clock {
        period(clock)?;
    }
    let state = Rc::new(RefCell::new(ResourceState::default()));
    macro_rules! collection {
        ($wrapper:ident, $field:ident) => {
            $wrapper(Collection {
                metadata: board.resources.$field,
                state: state.clone(),
            })
        };
    }
    Ok(Platform {
        board,
        leds: collection!(Leds, led),
        buttons: collection!(Buttons, button),
        switches: collection!(Switches, switch),
        resets: collection!(Resets, reset),
        uarts: collection!(Uarts, uart),
        clocks: collection!(Clocks, clock),
        default_clock_period,
    })
}

impl ResourceState {
    // Validate the entire resource before reserving anything, including UART aliases.
    fn reserve(&mut self, kind: &'static str, index: usize, pins: &[board::Pin]) -> Result<()> {
        let owner = ir::io::design_id();
        ensure!(
            self.owner.is_none_or(|id| id == owner),
            "resources belong to a different elaboration"
        );
        ensure!(
            !self.requested.contains(&(kind, index)),
            "resource {kind} {index} already taken"
        );
        let mut selected = HashSet::new();
        for pin in pins {
            ensure!(
                !pin.name.is_empty() && !pin.io_standard.is_empty(),
                "empty package pin or IO standard"
            );
            ensure!(
                selected.insert(pin.name),
                "duplicate package pin {} within resource",
                pin.name
            );
            ensure!(
                !self.pins.contains(pin.name),
                "package pin {} already taken through an alias",
                pin.name
            );
        }
        self.owner = Some(owner);
        self.requested.insert((kind, index));
        self.pins.extend(selected);
        Ok(())
    }

    fn bind(&mut self, signal: ir::SignalId, pin: board::Pin) {
        self.bindings.push(PinBinding {
            signal,
            bit: 0,
            package_pin: pin.name.into(),
            io_standard: pin.io_standard.into(),
        });
    }
}

impl<T> Collection<T> {
    fn get(&self, index: usize) -> Result<&T> {
        self.metadata.get(index).ok_or_else(|| {
            anyhow!(
                "resource index {index} out of range (length {})",
                self.metadata.len()
            )
        })
    }
}

macro_rules! scalar {
    ($metadata:ident, $kind:literal, $flow:ident, $allocate:ident) => {
        impl Collection<board::$metadata> {
            #[track_caller]
            pub(super) fn take(&self, index: usize) -> Result<$flow<bool>> {
                let metadata = self.get(index)?;
                let mut state = self.state.borrow_mut();
                state.reserve($kind, index, &[metadata.pin])?;
                let (port, signal) = ir::io::$allocate(
                    &format!("{}_{}", $kind, index),
                    metadata.active_on == board::ActiveOn::Low,
                );
                state.bind(signal, metadata.pin);
                Ok(port)
            }
        }
    };
}
scalar!(Led, "led", Out, output);
scalar!(Button, "button", In, input);
scalar!(Switch, "switch", In, input);
scalar!(Reset, "reset", In, input);

impl Collection<board::Uart> {
    #[track_caller]
    pub(super) fn take(&self, index: usize) -> Result<UartPort> {
        let metadata = self.get(index)?;
        let mut state = self.state.borrow_mut();
        state.reserve("uart", index, &[metadata.tx, metadata.rx])?;
        let (tx, tx_id) = ir::io::output(&format!("uart_{index}_tx"), false);
        let (rx, rx_id) = ir::io::input(&format!("uart_{index}_rx"), false);
        state.bind(tx_id, metadata.tx);
        state.bind(rx_id, metadata.rx);
        Ok(UartPort { tx, rx })
    }
}

impl Collection<board::Clock> {
    #[track_caller]
    pub(super) fn take(&self, index: usize) -> Result<Signal<bool>> {
        let metadata = self.get(index)?;
        let period = period(metadata)?;
        let mut state = self.state.borrow_mut();
        state.reserve("clock", index, &[metadata.pin])?;
        let (clock, signal) = ir::io::clock(&format!("clock_{index}"));
        state.bind(signal, metadata.pin);
        state.clocks.push(ClockBinding { signal, period });
        Ok(clock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform() -> Platform {
        Platform::from_board(board::arty_a7::A7_35T).unwrap()
    }

    #[test]
    fn metadata_only_conversion() {
        let p = platform();
        assert_eq!(p.default_clock_period.hertz(), 100_000_000);
        assert!(p.bindings().is_empty());
        assert!(p.clock_bindings().is_empty());
        let mut board = p.board;
        board.resources.ddr3 = board::arty_a7::A7_35T.resources.ddr3;
        assert!(Platform::from_board(board).is_ok());
        board.resources.clock = &[];
        assert!(Platform::from_board(board).is_err());
        board.resources.clock = &[board::Clock {
            hz: 0,
            pin: board::Pin {
                name: "C",
                io_standard: "LVCMOS33",
            },
        }];
        assert!(Platform::from_board(board).is_err());
    }

    #[test]
    fn duplicates_clones_and_owner() {
        let p = platform();
        let clone = p.clone();
        let leds = p.leds.clone();
        ir::elaborate_named("top", || {
            p.leds.take(0);
            assert!(clone.leds.try_take(0).is_err());
            assert!(leds.try_take(0).is_err());
            assert!(clone.buttons.try_take(999).is_err());
            clone.buttons.take(0);
            clone.switches.take(0);
            clone.resets.take(0);
            clone.uarts.take(0);
            assert!(p.uarts.try_take(0).is_err());
        })
        .unwrap();
        assert_eq!(p.bindings(), clone.bindings());
        let mut snapshot = p.bindings();
        snapshot.clear();
        assert_eq!(p.bindings().len(), 6);
        ir::elaborate_named("other", || {
            assert!(clone.leds.try_take(1).is_err());
            assert!(clone.clocks.try_take(0).is_err());
        })
        .unwrap();
    }

    #[test]
    fn uart_alias_validation_is_atomic() {
        let mut board = platform().board;
        board.resources.uart = &[
            board::Uart {
                tx: board::Pin {
                    name: "H5",
                    io_standard: "LVCMOS33",
                },
                rx: board::Pin {
                    name: "H5",
                    io_standard: "LVCMOS33",
                },
            },
            board::Uart {
                tx: board::Pin {
                    name: "FREE",
                    io_standard: "LVCMOS33",
                },
                rx: board::Pin {
                    name: "H5",
                    io_standard: "LVCMOS33",
                },
            },
        ];
        let p = Platform::from_board(board).unwrap();
        ir::elaborate_named("top", || {
            assert!(p.uarts.try_take(0).is_err());
            assert!(p.bindings().is_empty());
            p.leds.take(0);
            assert!(p.uarts.try_take(1).is_err());
            assert_eq!(p.bindings().len(), 1);
            assert!(!p.leds.0.state.borrow().pins.contains("FREE"));
        })
        .unwrap();
    }

    #[test]
    fn nested_top_level_polarity_and_clock_metadata() {
        let mut board = platform().board;
        board.resources.led = &[board::Led {
            pin: board::Pin {
                name: "LED",
                io_standard: "LVCMOS33",
            },
            active_on: board::ActiveOn::Low,
        }];
        board.resources.button = &[board::Button {
            pin: board::Pin {
                name: "BUTTON",
                io_standard: "LVCMOS33",
            },
            active_on: board::ActiveOn::Low,
        }];
        let p = Platform::from_board(board).unwrap();
        let (design, ()) = ir::elaborate_named("top", || {
            let _top = core::__::enter_module("top");
            let _nested = core::__::enter_module("nested");
            let mut led = p.leds.take(0);
            let button = p.buttons.take(0);
            led <<= button;
            p.clocks.take(0);
            assert!(p.clocks.try_take(0).is_err());
        })
        .unwrap();
        let bindings = p.bindings();
        for binding in &bindings {
            assert!(design.signals[binding.signal.0].port.is_some());
            assert!(!design.signals[binding.signal.0].name.contains('.'));
        }
        assert_eq!(p.clock_bindings()[0].signal, bindings[2].signal);
        assert_eq!(p.clock_bindings()[0].period.femtos(), 10_000_000);
        assert_eq!(
            design
                .values
                .iter()
                .filter(|value| matches!(
                    value.kind,
                    ir::ValueKind::Unary {
                        op: ir::UnaryOp::Not,
                        ..
                    }
                ))
                .count(),
            2
        );
        let mut sim = core::sim::Simulator::from_design(design).unwrap();
        for high in [false, true] {
            let mut bits = ir::PackedBits::zero(1);
            bits.set_bit(0, high);
            sim.set_signal(bindings[1].signal, bits).unwrap();
            assert_eq!(sim.get_signal(bindings[0].signal).unwrap().bit(0), high);
        }
    }
}
