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
    fn clock<D: Domain>(&self) -> Option<(Clock<D>, NonZero<u64>)> {
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
    /// Claims the LED pin; the domain is chosen when it is driven, via
    /// [`Led::output`].
    #[must_use]
    pub fn take(&self, index: usize) -> Led {
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
        self.context.bindings.borrow_mut().led.push(Binding {
            resource,
            name: name.clone(),
        });
        Led { resource, name }
    }
}

#[derive(Debug, Clone)]
pub struct Led {
    resource: board::Led,
    name: String,
}

impl Led {
    /// Creates the output register driving this pin in the given clock
    /// domain.
    #[must_use]
    pub fn output<D: Domain>(&self, clock: Clock<D>) -> Output<D> {
        Output::new(
            clock,
            &self.name,
            self.resource.active_on == board::ActiveOn::Low,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Buttons {
    resources: &'static [board::Button],
    context: Rc<ResourceContext>,
}

impl Buttons {
    /// Claims the button pin; the domain is chosen when it is read, via
    /// [`Button::input`].
    #[must_use]
    pub fn take(&self, index: usize) -> Button {
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
        self.context.bindings.borrow_mut().button.push(Binding {
            resource,
            name: name.clone(),
        });
        Button { resource, name }
    }
}

#[derive(Debug, Clone)]
pub struct Button {
    resource: board::Button,
    name: String,
}

impl Button {
    /// Creates the two-register synchronizer for this pin in the given
    /// clock domain.
    #[must_use]
    pub fn input<D: Domain>(&self, clock: Clock<D>) -> Value<bool> {
        io::input(
            clock,
            &self.name,
            self.resource.active_on == board::ActiveOn::Low,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoardClock<D: Domain> {
    pub clock: Clock<D>,
    pub hz: NonZero<u64>,
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub board: Board,
    pub led: Leds,
    pub button: Buttons,
    context: Rc<ResourceContext>,
}

impl Platform {
    /// The board's default clock, branded with the caller's domain
    /// marker. A board either has a default clock with its frequency or
    /// neither, so they are a single value.
    #[must_use]
    pub fn clock<D: Domain>(&self) -> BoardClock<D> {
        self.try_clock().expect("board has no default clock")
    }

    #[must_use]
    pub fn try_clock<D: Domain>(&self) -> Option<BoardClock<D>> {
        let (clock, hz) = self.context.clock::<D>()?;
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
