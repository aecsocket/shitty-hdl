use anyhow::Result;

use crate::{
    module::{In, Out},
    util::Period,
};

#[non_exhaustive]
pub struct Platform {
    pub resources: Resources,
    pub default_clock_period: Period,
}

impl Resources {
    pub fn request<R: Resource>(&mut self, resource: R) -> R::Out {
        resource.request(self).expect("requesting resource")
    }
}

pub struct Resources {}

pub trait Resource {
    type Out;

    fn request(self, resources: &mut Resources) -> Result<Self::Out>;
}

pub struct Led(pub usize);

impl Resource for Led {
    type Out = Out<bool>;

    fn request(self, resources: &mut Resources) -> Result<Self::Out> {
        todo!();
    }
}

pub struct Button(pub usize);

impl Resource for Button {
    type Out = In<bool>;

    fn request(self, resources: &mut Resources) -> Result<Self::Out> {
        todo!();
    }
}
