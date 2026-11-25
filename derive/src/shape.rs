use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Path, Result, ext::IdentExt, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(shape))]
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
    let data = match &input.data {
        Data::Struct(data) => data,
        Data::Enum(data) => {
            for variant in &data.variants {
                if !variant.fields.is_empty() {
                    return Err(syn::Error::new_spanned(
                        &variant.fields,
                        "Shape can only be derived for enum variants without fields",
                    ));
                }
            }
            let ident = &input.ident;
            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
            return Ok(quote! {
                impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
                    type View = #crate_::value::ValueView<Self>;
                }

                impl #impl_generics #crate_::value::IntoValues for #ident #ty_generics #where_clause {
                    type Out = Self;
                    type Values = #crate_::value::Value<Self>;

                    fn into_values(self) -> Self::Values {
                        #crate_::value::IntoValue::into_value(self)
                    }
                }
            });
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input,
                "Shape can only be derived for structs or enums without fields",
            ));
        }
    };
    let ident = &input.ident;
    let view_ident = format_ident!("{}View", ident.unraw());
    let vis = &input.vis;
    let mut generics = input.generics.clone();
    for field in &data.fields {
        let ty = &field.ty;
        generics.make_where_clause().predicates.push(parse_quote! {
            #ty: #crate_::value::Shape
        });
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fields = data.fields.iter().map(|field| {
        let ty = &field.ty;
        let vis = &field.vis;
        let view_ty = quote! { <#ty as #crate_::value::Shape>::View };
        match &field.ident {
            Some(ident) => quote! { #vis #ident: #view_ty },
            None => quote! { #vis #view_ty },
        }
    });
    let view = match &data.fields {
        Fields::Named(_) => quote! {
            #vis struct #view_ident #generics #where_clause {
                #(#fields,)*
            }
        },
        Fields::Unnamed(_) => quote! {
            #vis struct #view_ident #generics (
                #(#fields,)*
            ) #where_clause;
        },
        Fields::Unit => quote! {
            #vis struct #view_ident #generics #where_clause;
        },
    };
    Ok(quote! {
        #view

        impl #impl_generics ::core::marker::Copy for #view_ident #ty_generics #where_clause {}

        impl #impl_generics ::core::clone::Clone for #view_ident #ty_generics #where_clause {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
            type View = #view_ident #ty_generics;
        }

        impl #impl_generics #crate_::value::IntoValues for #ident #ty_generics #where_clause {
            type Out = Self;
            type Values = #crate_::value::Value<Self>;

            fn into_values(self) -> Self::Values {
                #crate_::value::IntoValue::into_value(self)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_structs() {
        for input in [
            quote! { enum Choice { A, B } },
            quote! { union Either { a: u8, b: u16 } },
        ] {
            let error = derive_(input).unwrap_err();
            assert!(error.to_string().contains("only be derived for structs"));
        }
    }

    #[test]
    fn rejects_removed_arithmetic_options() {
        for option in ["add", "sub", "mul", "div"] {
            let option = format_ident!("{option}");
            let error = derive_(quote! {
                #[shape(#option)]
                struct Wrapper(u8);
            })
            .unwrap_err();
            assert!(error.to_string().contains("Unknown field"), "{error}");
        }
    }

    #[test]
    fn expansion_is_valid_rust() {
        for input in [
            quote! { pub struct Named<T = bool> where T: Clone { pub field: T } },
            quote! { pub struct Tuple<T = bool>(pub T) where T: Clone; },
            quote! { pub struct Unit; },
            quote! { struct r#type { flag: bool } },
        ] {
            let output = derive_(input).unwrap();
            syn::parse2::<syn::File>(output).unwrap();
        }
    }
}
