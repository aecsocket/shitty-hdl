mod arith;
mod bits;
mod cmp;
mod hdl;
mod interface;
mod module;
mod shape;
mod shift;

fn root() -> syn::Path {
    syn::parse_quote! { ::shitty_hdl }
}

#[proc_macro_attribute]
pub fn module(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    module::derive(attr.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn hdl(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    hdl::derive(attr.into(), input.into()).into()
}

#[proc_macro_derive(Interface, attributes(interface))]
pub fn interface(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    interface::derive(input.into()).into()
}

#[proc_macro_derive(BitCmp, attributes(bit_cmp))]
pub fn bit_cmp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cmp::bit_cmp(input.into()).into()
}

#[proc_macro_derive(ArithCmp, attributes(arith_cmp))]
pub fn arith_cmp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cmp::arith_cmp(input.into()).into()
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
