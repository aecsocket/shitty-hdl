use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Path, Result, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(bits))]
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
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let widths = data.fields.iter().map(|field| {
        let ty = &field.ty;
        quote! { <#ty as #crate_::ops::Bits>::WIDTH }
    });
    let width = quote! { (0usize #(+ #widths)*) };
    if !input.generics.params.is_empty() {
        for field in &data.fields {
            let ty = &field.ty;
            generics.make_where_clause().predicates.push(parse_quote! {
                #ty: #crate_::ops::Bits
            });
        }
        let (_, original_ty_generics, _) = input.generics.split_for_impl();
        generics.make_where_clause().predicates.push(parse_quote! {
            #ident #original_ty_generics: #crate_::value::Shape
        });
        generics.make_where_clause().predicates.push(parse_quote! {
            [(); #width]:
        });
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #crate_::ops::Bits for #ident #ty_generics #where_clause {
            const WIDTH: usize = #width;
            type Packed = #crate_::value::Bit<{ #width }>;

            fn pack(this: #crate_::value::Value<Self>) -> #crate_::value::Value<Self::Packed> {
                let _ = this;
                ::core::todo!()
            }

            fn unpack(bits: #crate_::value::Value<Self::Packed>) -> #crate_::value::Value<Self> {
                let _ = bits;
                ::core::todo!()
            }
        }
    })
}
