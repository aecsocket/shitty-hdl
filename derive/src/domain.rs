use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Path, Result};

#[derive(FromDeriveInput)]
#[darling(attributes(domain))]
struct Options {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn derive(input: TokenStream) -> TokenStream {
    derive_(input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive_(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let options = Options::from_derive_input(&input)?;
    let crate_ = &options.crate_;
    match &input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Unit) => {}
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Domain can only be derived for unit structs",
            ));
        }
    }
    let name = &input.ident;
    Ok(quote! { impl #crate_::domain::Domain for #name {} })
}
