use std::any::TypeId;

use crate::elaborate::with_elaboration;

pub struct Clock(());

impl Clock {
    pub fn on(&self, f: impl FnMut()) {}
}

pub fn comb(f: impl FnOnce()) {
    in_domain::<Comb>(f);
}

pub struct Sync;
pub struct Comb;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(TypeId);

impl DomainId {
    pub fn of<D: ?Sized + 'static>() -> Self {
        Self(TypeId::of::<D>())
    }
}

pub fn in_domain<D: 'static>(f: impl FnOnce()) {
    let domain_id = DomainId::of::<D>();
    with_elaboration(|cx| cx.in_domain(domain_id, f))
}
