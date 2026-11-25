use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(error) => return error.to_compile_error(),
    };
    let crate_ = quote! { shitty_hdl };
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #crate_::__::Shape for #ident #ty_generics #where_clause {}
    }
}
