use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DESIGN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DesignId(u64);

impl DesignId {
    pub(crate) fn new() -> Self {
        let id = NEXT_DESIGN_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("elaboration design identity overflow");
        Self(id)
    }
}
