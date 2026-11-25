use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Path, Result, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(bit_cmp))]
struct BitOptions {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

#[derive(FromDeriveInput)]
#[darling(attributes(arith_cmp))]
struct ArithOptions {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn bit_cmp(input: TokenStream) -> TokenStream {
    derive(input, false).unwrap_or_else(|error| error.to_compile_error())
}

pub fn arith_cmp(input: TokenStream) -> TokenStream {
    derive(input, true).unwrap_or_else(|error| error.to_compile_error())
}

fn derive(input: TokenStream, arithmetic: bool) -> Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let crate_ = if arithmetic {
        ArithOptions::from_derive_input(&input)?.crate_
    } else {
        BitOptions::from_derive_input(&input)?.crate_
    };
    let trait_ident = format_ident!("{}", if arithmetic { "ArithCmp" } else { "BitCmp" });
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "can only be derived for structs",
        ));
    };
    if data.fields.len() != 1 {
        return Err(syn::Error::new_spanned(
            &input,
            format!("{trait_ident} requires exactly one field"),
        ));
    }
    let ty = &data.fields.iter().next().unwrap().ty;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ty: #crate_::ops::#trait_ident + #crate_::ops::Bits
    });
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: #crate_::ops::Bits<
            Packed = <#ty as #crate_::ops::Bits>::Packed
        >
    });
    if arithmetic {
        generics.make_where_clause().predicates.push(parse_quote! {
            #ident #original_ty_generics: #crate_::ops::Arith
        });
    }
    let names: &[&str] = if arithmetic {
        &["gt", "ge", "lt", "le"]
    } else {
        &["eq", "ne"]
    };
    let methods = names.iter().map(|name| {
        let name = format_ident!("{name}");
        quote! {
            fn #name(
                lhs: #crate_::value::Value<Self>,
                rhs: #crate_::value::Value<Self>,
            ) -> #crate_::value::Value<bool> {
                let lhs = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(lhs));
                let rhs = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(rhs));
                <#ty as #crate_::ops::#trait_ident>::#name(lhs, rhs)
            }
        }
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #crate_::ops::#trait_ident for #ident #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
