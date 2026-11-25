use crate::{signal::Signal, util::Period};

#[non_exhaustive]
pub struct Platform {
    pub default_clock_period: Period,
}

impl Platform {
    pub fn request<R: Resource>(&mut self, resource: R) -> R::Out {
        todo!();
    }
}

pub trait Resource {
    type Out;
}

pub struct Led;

impl Resource for Led {
    type Out = Signal<bool>;
}

pub struct Button;

impl Resource for Button {
    type Out = Signal<bool>;
}
