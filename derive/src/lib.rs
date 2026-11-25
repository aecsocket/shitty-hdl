mod arith;
mod bits;
mod interface;
mod module;
mod shape;
mod shift;

fn root() -> syn::Path {
    syn::parse_quote! { ::shitty_hdl }
}

/// Turns this function into a `Module`, which can be instantiated as a hardware
/// circuit.
///
/// Modules are a way to describe a specific, discrete part of a hardware
/// design. They take a set of input and output ports, and describe how those
/// ports are wired up to each other, and to internal registers in the module.
///
/// # Definition
///
/// A simple module which takes an 8-bit number, and sets another signal to
/// high if it is above 100, is described like so:
///
/// ```
/// #[module]
/// fn my_counter(number: In<Uint<U8>>, is_above_100: Out<bool>) {
///     comb(|| is_above_100 <<= number.gt(u!(100)));
/// }
/// ```
///
/// Let's break the module down:
/// - `In<Uint<U8>>` indicates an _input port_ which takes an 8-bit unsigned
///   integer; the module can only read from this, and cannot write to it.
/// - `Out<bool>` indicates an _output port_, which expects the module to drive
///   it with a `bool` value.
/// - `comb(|| ..)` indicates that we want to describe some _combinational
///   logic_; this is logic that is not tied to any particular clock. See `comb`
///   for more details.
/// - `is_above_100 <<= ..` describes writing a value to the `is_above_100`
///   output port.
/// - `number.gt(..)` is how you perform a greater-than comparison between
///   `number` and some other expression.
/// - `u!(100)` is syntax for creating a `Uint<_>`; so we're saying that we want
///   to check if the current value of `number` is greater than `100`.
///
/// The above module uses only combinational logic (`comb`), which means it
/// isn't tied to the clock at all. Modules can also specify clocked logic;
/// for example, this module will create an 8-bit counter, and increment it
/// on each clock tick:
///
/// ```
/// #[module]
/// fn accumulator(sync: Domain<Sync>) {
///     let count = Signal::<Uint<U8>>::new(u!(0));
///     sync.on(|| count <<= count + u!(1));
/// }
/// ```
///
/// To perform assignments in a clocked domain, use `my_domain.on(|| ..)`. In
/// a clocked domain, you can describe assignments to signals which depend on
/// the previous state of that same signal.
// TODO document more
#[proc_macro_attribute]
pub fn module(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    module::derive(attr.into(), input.into()).into()
}

#[proc_macro_derive(Interface, attributes(interface))]
pub fn interface(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    interface::derive(input.into()).into()
}

#[proc_macro_derive(Arith, attributes(arith))]
pub fn arith(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    arith::derive(input.into()).into()
}

#[proc_macro_derive(Bits, attributes(bits))]
pub fn bits(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bits::derive(input.into()).into()
}

#[proc_macro_derive(Shift, attributes(shift))]
pub fn shift(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shift::derive(input.into()).into()
}

#[proc_macro_derive(Shape, attributes(shape))]
pub fn shape(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shape::derive(input.into()).into()
}
