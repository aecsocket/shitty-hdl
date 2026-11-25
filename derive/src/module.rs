use proc_macro2::TokenStream;

pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    crate::hdl::derive(attr, input)
}
