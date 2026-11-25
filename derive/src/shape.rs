use darling::{FromDeriveInput, util::Flag};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Result, parse_quote};

#[derive(Default, FromDeriveInput)]
#[darling(attributes(shape))]
struct Options {
    add: Flag,
    sub: Flag,
    mul: Flag,
    div: Flag,
    #[darling(rename = "crate")]
    crate_: Option<syn::Path>,
}

pub fn derive(input: TokenStream) -> TokenStream {
    derive_(input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive_(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let options = Options::from_derive_input(&input)?;
    let crate_ = options
        .crate_
        .unwrap_or_else(|| parse_quote! { shitty_hdl });
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut impls = quote! {
        impl #impl_generics #crate_::__::Shape for #ident #ty_generics #where_clause {}
    };

    let operations = [
        (options.add, "ValAdd"),
        (options.sub, "ValSub"),
        (options.mul, "ValMul"),
        (options.div, "ValDiv"),
    ];
    if !operations.iter().any(|(flag, _)| flag.is_present()) {
        return Ok(impls);
    }

    let field = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => &fields.unnamed[0],
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "shape arithmetic options require a tuple struct with exactly one field",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "shape arithmetic options require a tuple struct with exactly one field",
            ));
        }
    };
    let inner = &field.ty;

    for (flag, name) in operations {
        if !flag.is_present() {
            continue;
        }
        let trait_ = format_ident!("{name}");
        let mut generics = input.generics.clone();
        generics.make_where_clause().predicates.push(parse_quote! {
            #inner: #crate_::__::#trait_<#inner>
        });
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        impls.extend(quote! {
            impl #impl_generics #crate_::__::#trait_<Self> for #ident #ty_generics #where_clause {}
        });
    }

    Ok(impls)
}
