mod stack;

use std::sync::atomic::{self, AtomicBool, AtomicU32};

use pir::Bits;

use crate::{elab::stack::ModuleStack, signal::SignalId, value::Shape};

pub fn elaborate(f: impl FnOnce()) -> Module {
    struct ElaborateGuard;

    static ELABORATING: AtomicBool = AtomicBool::new(false);

    impl ElaborateGuard {
        fn enter() -> Self {
            let already_elaborating = ELABORATING.swap(true, atomic::Ordering::SeqCst);
            assert!(
                !already_elaborating,
                "attempted to enter elaboration while already elaborating"
            );
            Self
        }
    }

    impl Drop for ElaborateGuard {
        fn drop(&mut self) {
            ELABORATING.store(false, atomic::Ordering::SeqCst);
        }
    }

    let _guard = ElaborateGuard::enter();
    let design_id = DesignId(NEXT_DESIGN_ID.fetch_add(1, atomic::Ordering::Relaxed));
    let guard = ModuleStack::enter(design_id);
    f();
    drop(guard);

    // pir::Module {
    //     name
    // }

    todo!();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignId(u32);

static NEXT_DESIGN_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleId {
    design_id: DesignId,
    id: u32,
}

#[derive(Debug)]
pub struct Module {
    id: ModuleId,
    signals: Vec<SignalState>,
}

#[derive(Debug)]
struct SignalState {
    init: Option<Bits>,
}

impl Module {
    const fn new(id: ModuleId) -> Self {
        Self {
            id,
            signals: Vec::new(),
        }
    }
}

pub(crate) fn make_signal<T: Shape>(init: Option<T>) -> SignalId {
    ModuleStack::with_current(|module| {
        let id = u32::try_from(module.signals.len()).expect("too many signals");
        let id = SignalId {
            design_id: module.id.design_id,
            id,
        };
        let init = init.map(T::into_bits);
        module.signals.push(SignalState { init });
        id
    })
}
