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
pub fn comb(f: impl FnOnce()) {}

/// Signal which consistently raises and lowers over time, following a period.
///
/// A clock allows you to define logic which only updates once every time the
/// clock goes high (on each positive edge).
///
/// [`Clock`] is the default synchronous clocked domain, which should be used
/// for the "default" synchronous logic in your design. You can also create your
/// own clock signals by deriving or implementing [`Domain`]:
///
/// ```
/// #[derive(Domain)]
/// struct AdcClock(());
/// ```
pub struct Clock(());
impl Domain for Clock {}

/// Marker trait for a clock signal input, like [`Clock`].
///
/// Domains are uniquely identified by their [`type_id`]; this trait just exists
/// to make sure you don't try to `DomainExt::on(val)` where `val` isn't a
/// [`Domain`].
///
/// [`type_id`]: std::any::TypeId
pub trait Domain {}

pub trait DomainExt: Domain {
    fn on(&mut self, f: impl FnOnce());
}
impl<D: Domain> DomainExt for D {
    fn on(&mut self, f: impl FnOnce()) {}
}
