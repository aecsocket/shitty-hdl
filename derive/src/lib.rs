mod module;
mod shape;

#[proc_macro_attribute]
pub fn module(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    module::derive(attr.into(), input.into()).into()
}

/// Derives a packed shape for a named, tuple, or unit struct.
///
/// Generates a sibling `NameView` whose fields use each field shape's `View`,
/// preserving visibility and generics. The view is `Copy` and `Clone`; `WIDTH`
/// is the sum of field widths. Use `#[shape(crate = "path")]` to override the
/// default `shitty_hdl` crate path.
#[proc_macro_derive(Shape, attributes(shape))]
pub fn shape(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shape::derive(input.into()).into()
}
