use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Path, Result, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(shift))]
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
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "can only be derived for structs",
        ));
    };
    if data.fields.len() != 1 {
        return Err(syn::Error::new_spanned(
            &input,
            "Shift requires exactly one field",
        ));
    }
    let ty = &data.fields.iter().next().unwrap().ty;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ty: #crate_::ops::Shift
    });
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: #crate_::ops::Bits<
            Packed = <#ty as #crate_::ops::Bits>::Packed
        >
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #crate_::ops::Shift for #ident #ty_generics #where_clause {
            fn shl(value: #crate_::value::Value<Self>, amount: usize) -> #crate_::value::Value<Self> {
                let value = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(value));
                let result = <#ty as #crate_::ops::Shift>::shl(value, amount);
                <Self as #crate_::ops::Bits>::unpack(
                    <#ty as #crate_::ops::Bits>::pack(result))
            }

            fn shr(value: #crate_::value::Value<Self>, amount: usize) -> #crate_::value::Value<Self> {
                let value = <#ty as #crate_::ops::Bits>::unpack(
                    <Self as #crate_::ops::Bits>::pack(value));
                let result = <#ty as #crate_::ops::Shift>::shr(value, amount);
                <Self as #crate_::ops::Bits>::unpack(
                    <#ty as #crate_::ops::Bits>::pack(result))
            }
        }
    })
}
