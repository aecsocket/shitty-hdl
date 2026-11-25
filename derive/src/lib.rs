mod module;
mod shape;

#[proc_macro_attribute]
pub fn module(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    module::derive(attr.into(), input.into()).into()
}

#[proc_macro_derive(Shape)]
pub fn shape(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shape::derive(input.into()).into()
}
