pub use board;

use board::Board;
use front::{
    domain::{Clock, Domain},
    io,
    io::Output,
    value::Value,
};
use std::{cell::RefCell, num::NonZero, rc::Rc};

#[derive(Debug, Clone, Default)]
pub struct Bindings {
    pub clock: Vec<ClockBinding>,
    pub reset: Vec<Binding<board::Reset>>,
    pub led: Vec<Binding<board::Led>>,
    pub button: Vec<Binding<board::Button>>,
}

pub type ClockBinding = Binding<board::Clock>;

#[derive(Debug, Clone)]
pub struct Binding<T> {
    pub resource: T,
    pub name: String,
}

#[derive(Debug)]
struct ResourceContext {
    clock: Option<board::Clock>,
    reset: Option<board::Reset>,
    bindings: RefCell<Bindings>,
}

impl ResourceContext {
    fn default_clock(&self) -> Option<(Clock, NonZero<u64>)> {
        let resource = self.clock?;
        let clock = Clock::input("clk");
        let mut bindings = self.bindings.borrow_mut();
        if !bindings.clock.iter().any(|binding| binding.name == "clk") {
            bindings.clock.push(Binding {
                resource,
                name: "clk".into(),
            });
        }
        if let Some(resource) = self.reset
            && bindings.reset.is_empty()
        {
            bindings.reset.push(Binding {
                resource,
                name: "reset".into(),
            });
        }
        Some((clock, resource.hz))
    }
}

#[derive(Debug, Clone)]
pub struct Leds {
    resources: &'static [board::Led],
    context: Rc<ResourceContext>,
}

impl Leds {
    #[must_use]
    pub fn take(&self, index: usize) -> Output {
        let resource = *self.resources.get(index).expect("LED index out of range");
        let name = format!("led_{index}");
        assert!(
            !self
                .context
                .bindings
                .borrow()
                .led
                .iter()
                .any(|binding| binding.name == name),
            "LED resource is already taken"
        );
        let clock = self
            .context
            .default_clock()
            .expect("LEDs require a default clock")
            .0;
        let output = Output::new(clock, &name, resource.active_on == board::ActiveOn::Low);
        self.context
            .bindings
            .borrow_mut()
            .led
            .push(Binding { resource, name });
        output
    }
}

#[derive(Debug, Clone)]
pub struct Buttons {
    resources: &'static [board::Button],
    context: Rc<ResourceContext>,
}

impl Buttons {
    #[must_use]
    pub fn take(&self, index: usize) -> Value<bool> {
        let resource = *self
            .resources
            .get(index)
            .expect("button index out of range");
        let name = format!("button_{index}");
        assert!(
            !self
                .context
                .bindings
                .borrow()
                .button
                .iter()
                .any(|binding| binding.name == name),
            "button resource is already taken"
        );
        let clock = self
            .context
            .default_clock()
            .expect("buttons require a default clock")
            .0;
        let input = io::input(clock, &name, resource.active_on == board::ActiveOn::Low);
        self.context
            .bindings
            .borrow_mut()
            .button
            .push(Binding { resource, name });
        input
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoardClock {
    pub clock: Clock,
    pub hz: NonZero<u64>,
}

impl Domain for BoardClock {
    fn clock(self) -> Clock {
        self.clock
    }
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub board: Board,
    pub led: Leds,
    pub button: Buttons,
    context: Rc<ResourceContext>,
}

impl Platform {
    /// The default clock together with its frequency; a board either has
    /// both or neither, so they are a single value.
    #[must_use]
    pub fn default_clock(&self) -> BoardClock {
        self.try_default_clock()
            .expect("design requires a default clock")
    }

    #[must_use]
    pub fn try_default_clock(&self) -> Option<BoardClock> {
        let (clock, hz) = self.context.default_clock()?;
        Some(BoardClock { clock, hz })
    }

    #[must_use]
    pub fn bindings(&self) -> Bindings {
        self.context.bindings.borrow().clone()
    }
}

#[must_use]
pub fn from_board(board: Board) -> Platform {
    let default_clock = board
        .resources
        .default_clock
        .and_then(|index| board.resources.clock.get(index));

    let context = Rc::new(ResourceContext {
        clock: default_clock.copied(),
        reset: board
            .resources
            .default_reset
            .and_then(|index| board.resources.reset.get(index))
            .copied(),
        bindings: RefCell::default(),
    });
    Platform {
        board,
        led: Leds {
            resources: board.resources.led,
            context: Rc::clone(&context),
        },
        button: Buttons {
            resources: board.resources.button,
            context: Rc::clone(&context),
        },
        context,
    }
}
