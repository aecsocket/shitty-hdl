use std::marker::PhantomData;

/// Perform combinational updates to signals.
///
/// Combinational logic works by updating the output immediately after its
/// inputs change; there is no clock signal to synchronize on. This is in
/// contrast to using [`Clock::on`], which will wait until the next rising
/// edge of the clock to update the logic. For example, the following logic
/// will define `sum` to be driven combinationally by always setting it to
/// `a + b`:
///
/// ```
/// fn wrapping_add(a: In<u32>, b: In<u32>, sum: Out<u32>) {
///     comb(|| sum <<= a + b);
/// }
/// ```
///
/// Once `a` or `b` change, `sum` will change immediately (well, a few
/// picoseconds) afterwards.
///
/// Setting a [signal] to be driven by combinational logic means that there
/// _must not_ be cycles in the combinational logic. For example, the following
/// would raise an error during elaboration:
///
/// ```
/// fn add_one_to_self() {
///     let count = Signal::<u32>::new();
///     comb(|| count <<= count + 1);
/// }
/// ```
///
/// In cases where state must depend on previous state, use clocked logic such
/// as [`Clock::on`] instead:
///
/// ```
/// fn add_one_to_self(clk: Clock) {
///     let count = Signal::<u32>::new();
///     clk.on(|| count <<= count + 1);
/// }
/// ```
pub fn comb(f: impl FnOnce()) {
    let _scope = crate::ir::runtime::domain(crate::ir::WriteDomain::Comb);
    f();
}

pub trait DomainLabel: 'static {}

pub struct Sync(());
impl DomainLabel for Sync {}

pub struct Domain<L: DomainLabel> {
    _phantom: PhantomData<L>,
}

impl<L: DomainLabel> Domain<L> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    pub fn id(&self) -> crate::ir::DomainId {
        crate::ir::DomainId::of::<L>()
    }

    #[track_caller]
    pub fn on(&mut self, f: impl FnOnce()) {
        let id = crate::ir::runtime::type_domain::<L>();
        let _scope = crate::ir::runtime::domain(crate::ir::WriteDomain::Clocked(id));
        f();
    }
}

impl<L: DomainLabel> Default for Domain<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: DomainLabel> Clone for Domain<L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: DomainLabel> Copy for Domain<L> {}
