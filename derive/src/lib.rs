mod arith;
mod display;
mod domain;
mod hdl;
mod interface;
mod module;
mod shape;

#[proc_macro]
pub fn display_format(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    display::display_format(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn displayln_format(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    display::displayln_format(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

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

#[proc_macro_derive(Domain, attributes(domain))]
pub fn domain(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    domain::derive(input.into()).into()
}

#[proc_macro_derive(Interface, attributes(interface))]
pub fn interface(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    interface::derive(input.into()).into()
}

#[proc_macro_derive(Arith, attributes(arith))]
pub fn arith(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    arith::derive(input.into()).into()
}

#[proc_macro_derive(Shape, attributes(shape))]
pub fn shape(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shape::derive(input.into()).into()
}
