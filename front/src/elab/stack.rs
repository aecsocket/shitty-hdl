use std::{
    cell::RefCell,
    sync::atomic::{self, AtomicU32},
};

use crate::elab::{DesignId, Module, ModuleId};

pub struct ModuleStack(());

static NEXT_MODULE_ID: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static STACK: RefCell<Vec<Module>> = const { RefCell::new(Vec::new()) };
}

impl ModuleStack {
    pub fn enter(design_id: DesignId) -> Self {
        let id = ModuleId {
            design_id,
            id: NEXT_MODULE_ID.fetch_add(1, atomic::Ordering::Relaxed),
        };
        STACK.with_borrow_mut(|stack| stack.push(Module::new(id)));
        Self(())
    }

    /// Provides access to the current top [`Module`] in the stack.
    ///
    /// Keep `f` short, and don't re-enter [`ModuleStack`] inside `f`.
    pub fn with_current<R>(f: impl FnOnce(&mut Module) -> R) -> R {
        STACK.with_borrow_mut(|stack| {
            let top = stack
                .last_mut()
                .expect("attempted to use elaboration function while not elaborating");
            f(top)
        })
    }
}

impl Drop for ModuleStack {
    fn drop(&mut self) {
        STACK.with_borrow_mut(|stack| {
            assert!(stack.pop().is_some(), "last stack item already popped");
        });
    }
}
