use std::num::NonZero;

use board::{ActiveOn, Board, Button, Led, Pin, Reset, Switch, Uart};
use front::{In, Out, Sync, util::Period};

use crate::{Platform, ResourceState, Resources, UartPort};

fn resources<T>(owner: ir::DesignId, items: impl IntoIterator<Item = T>) -> Resources<T> {
    Resources {
        items: items
            .into_iter()
            .map(|item| ResourceState::Held { item })
            .collect(),
        owner,
    }
}

#[track_caller]
fn input(name: &str, pin: Pin, active_on: ActiveOn) -> In<bool> {
    ir::io::check_pins(&[(pin.name, pin.io_standard)]).expect("invalid resource pin");
    let (port, raw) = ir::io::input(name, active_on == ActiveOn::Low);
    ir::io::bind_pin(raw, pin.name, pin.io_standard).expect("binding resource pin");
    port
}

#[track_caller]
pub(super) fn reset(resource: Reset, index: usize) -> In<bool> {
    input(&format!("reset_{index}"), resource.pin, resource.active_on)
}

#[track_caller]
pub(super) fn button(resource: Button, index: usize) -> In<bool> {
    input(&format!("button_{index}"), resource.pin, resource.active_on)
}

#[track_caller]
pub(super) fn switch(resource: Switch, index: usize) -> In<bool> {
    input(&format!("switch_{index}"), resource.pin, resource.active_on)
}

#[track_caller]
pub(super) fn led(resource: Led, index: usize) -> Out<bool> {
    let pin = resource.pin;
    ir::io::check_pins(&[(pin.name, pin.io_standard)]).expect("invalid LED pin");
    let (port, raw) = ir::io::output(&format!("led_{index}"), resource.active_on == ActiveOn::Low);
    ir::io::bind_pin(raw, pin.name, pin.io_standard).expect("binding LED pin");
    port
}

#[track_caller]
pub(super) fn uart(resource: Uart, index: usize) -> UartPort {
    let metadata = resource;
    ir::io::check_pins(&[
        (metadata.tx.name, metadata.tx.io_standard),
        (metadata.rx.name, metadata.rx.io_standard),
    ])
    .expect("invalid UART pins");
    let (tx, raw) = ir::io::output(&format!("uart_{index}_tx"), false);
    ir::io::bind_pin(raw, metadata.tx.name, metadata.tx.io_standard).expect("binding UART TX");
    let (rx, raw) = ir::io::input(&format!("uart_{index}_rx"), false);
    ir::io::bind_pin(raw, metadata.rx.name, metadata.rx.io_standard).expect("binding UART RX");
    UartPort { tx, rx }
}

pub(super) fn from_board(board: Board) -> Platform {
    let owner = ir::io::design_id();
    let default_clock = board.resources.default_clock.map(|index| {
        board
            .resources
            .clock
            .get(index)
            .expect("invalid default clock index")
    });
    if let Some(index) = board.resources.default_reset {
        assert!(
            index < board.resources.reset.len(),
            "invalid default reset index"
        );
    }
    let mut platform = Platform {
        board,
        default_clock_hz: default_clock.map(|clock| clock.hz),
        reset: resources(owner, board.resources.reset.iter().copied()),
        led: resources(owner, board.resources.led.iter().copied()),
        button: resources(owner, board.resources.button.iter().copied()),
        switch: resources(owner, board.resources.switch.iter().copied()),
        uart: resources(owner, board.resources.uart.iter().copied()),
        ddr3: resources(owner, board.resources.ddr3.iter().copied()),
    };
    if let Some(metadata) = default_clock {
        let period = Period::from_femtos(
            NonZero::new(1_000_000_000_000_000 / metadata.hz)
                .expect("clock period is below one femtosecond"),
        );
        let pin = metadata.pin;
        ir::io::check_pins(&[(pin.name, pin.io_standard)]).expect("invalid clock pin");
        let (clock, raw) =
            ir::io::clock(&format!("clock_{}", board.resources.default_clock.unwrap()));
        ir::io::bind_pin(raw, pin.name, pin.io_standard).expect("binding clock pin");
        ir::io::constrain_clock(raw, period).expect("constraining clock");
        ir::io::bind_domain_clock::<Sync>(clock.id()).expect("binding default clock");
        if let Some(index) = board.resources.default_reset {
            let reset = platform.reset.take(index);
            ir::io::bind_domain_reset::<Sync>(reset.id()).expect("binding default reset");
        }
    }
    platform
}
