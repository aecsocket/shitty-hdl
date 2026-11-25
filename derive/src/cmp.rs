use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Path, Result, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(arith_cmp))]
struct Options {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn arith_cmp(input: TokenStream) -> TokenStream {
    derive(input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let crate_ = &Options::from_derive_input(&input)?.crate_;
    let trait_ident = syn::Ident::new("BitCmp", proc_macro2::Span::call_site());
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
        #ty: #crate_::ops::BitEq + #crate_::ops::Bits
    });
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: #crate_::ops::Bits<
            Packed = <#ty as #crate_::ops::Bits>::Packed
        >
    });
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: #crate_::ops::Arith
    });
    let names: &[&str] = &["gt", "ge", "lt", "le"];
    let methods = names.iter().map(|name| {
        let name = syn::Ident::new(name, proc_macro2::Span::call_site());
        quote! {
            fn #name<D: #crate_::domain::Domain>(
                lhs: #crate_::value::Value<D, Self>,
                rhs: #crate_::value::Value<D, Self>,
            ) -> #crate_::value::Value<D, bool> {
                let lhs = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(lhs));
                let rhs = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(rhs));
                <#ty as #crate_::ops::BitCmp>::#name(lhs, rhs)
            }
        }
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #crate_::ops::BitCmp for #ident #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
