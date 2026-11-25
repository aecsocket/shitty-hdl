use std::marker::PhantomData;

use derive_more::Debug;

#[derive(Debug)]
pub struct Domain<L: DomainLabel> {
    _phantom: PhantomData<L>,
}

pub trait DomainLabel {}

pub struct Clock(());
impl DomainLabel for Clock {}

impl<L: DomainLabel> Domain<L> {
    pub fn on(&mut self, f: impl FnOnce()) {
        todo!();
    }
}
