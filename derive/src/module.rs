use darling::{FromMeta, util::Flag};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Path, Result, parse_quote};

#[derive(FromMeta)]
#[darling(derive_syn_parse)]
struct Options {
    builder: Flag,
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_(attr, input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive_(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let options = syn::parse2::<Options>(attr)?;
    let crate_ = &options.crate_;
    let mut function = syn::parse2::<ItemFn>(input)?;
    crate::hdl::rewrite(&mut function.block, crate_)?;
    let name = &function.sig.ident;
    let body = &function.block;
    *function.block = parse_quote! {
        { #crate_::__::module(stringify!(#name), || #body) }
    };
    function
        .attrs
        .push(parse_quote! { #[allow(clippy::future_not_send)] });
    if options.builder.is_present() {
        function.attrs.push(parse_quote! {
            #[#crate_::__::bon::builder(crate = #crate_::__::bon)]
        });
    }
    Ok(quote! { #function })
}
