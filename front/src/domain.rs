use std::any::TypeId;

use crate::elab;

pub trait Domain: Copy {
    fn clock(self) -> Clock;
}

#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub(crate) identity: TypeId,
    pub(crate) handle: Option<elab::ClockHandle>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            identity: TypeId::of::<Self>(),
            handle: None,
        }
    }
}

impl Clock {
    #[must_use]
    pub fn input(name: &str) -> Self {
        Self::input_for::<Self>(name)
    }

    #[must_use]
    pub fn input_for<D: 'static>(name: &str) -> Self {
        Self {
            identity: TypeId::of::<D>(),
            handle: Some(elab::clock_input::<D>(name)),
        }
    }
}

impl Domain for Clock {
    fn clock(self) -> Clock {
        self
    }
}
