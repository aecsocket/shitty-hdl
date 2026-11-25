use std::cell::RefCell;

use crate::{DomainId, Shape, Signal, SignalId};

#[derive(Debug)]
pub struct Elaboration {
    signals: Vec<SignalDef>,
}

#[derive(Debug)]
struct SignalDef {
    init: Option<Box<[u64]>>,
}

thread_local! {
    static ELABORATION: RefCell<Option<Elaboration>> = const { RefCell::new(None) };
}

pub fn with_elaboration<R>(f: impl FnOnce(&mut Elaboration) -> R) -> R {
    ELABORATION.with_borrow_mut(|cell| {
        let cell = cell
            .as_mut()
            .expect("attempted to access elaboration context while not elaborating");
        f(cell)
    })
}

impl Elaboration {
    pub fn new() -> Self {
        Self {
            signals: Vec::with_capacity(256),
        }
    }

    pub fn new_signal<T: Shape>(&mut self, init: Option<T>) -> Signal<T> {
        todo!();
        // let id = SignalId::from_raw(self.signals.len());
        // let signal = Signal::from_id(id);
        // self.signals.push(SignalDef { init });
    }

    pub fn in_domain(&mut self, domain_id: DomainId, f: impl FnOnce()) {}
}
