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
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "can only be derived for structs",
        ));
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
    // Generic defaults belong on the view declaration, but not on impls.
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
    let widths = data.fields.iter().map(|field| {
        let ty = &field.ty;
        quote! { <#ty as #crate_::value::Shape>::WIDTH }
    });
    let members: Vec<_> = data
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            field
                .ident
                .clone()
                .map(syn::Member::Named)
                .unwrap_or_else(|| syn::Member::Unnamed(syn::Index::from(index)))
        })
        .collect();
    let types: Vec<_> = data.fields.iter().map(|field| &field.ty).collect();
    let locals: Vec<_> = (0..types.len())
        .map(|i| format_ident!("__shape_field_{i}"))
        .collect();
    let construct = |values: Vec<TokenStream>| match &data.fields {
        Fields::Named(_) => quote! { { #(#members: #values,)* } },
        Fields::Unnamed(_) => quote! { ( #(#values,)* ) },
        Fields::Unit => quote! {},
    };
    let decoded = construct(locals.iter().map(|local| quote! { #local }).collect());
    let views = decoded.clone();
    Ok(quote! {
        #view

        impl #impl_generics ::core::marker::Copy for #view_ident #ty_generics #where_clause {}

        impl #impl_generics ::core::clone::Clone for #view_ident #ty_generics #where_clause {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
            const WIDTH: #crate_::value::ShapeSize = {
                let mut width: #crate_::value::ShapeSize = 0;
                #(width = width.checked_add(#widths).expect("Shape width overflow");)*
                width
            };
            type View = #view_ident #ty_generics;

            fn to_bits(&self) -> #crate_::ir::PackedBits {
                let mut bits = #crate_::ir::PackedBits::zero(<Self as #crate_::value::Shape>::WIDTH);
                let mut offset: #crate_::value::ShapeSize = 0;
                #(
                    bits.replace(offset, &<#types as #crate_::value::Shape>::to_bits(&self.#members));
                    offset = offset.checked_add(<#types as #crate_::value::Shape>::WIDTH)
                        .expect("Shape offset overflow");
                )*
                let _ = offset;
                bits
            }

            fn from_bits(bits: &#crate_::ir::PackedBits) -> Self {
                assert_eq!(bits.width(), <Self as #crate_::value::Shape>::WIDTH, "Shape width mismatch");
                let mut offset: #crate_::value::ShapeSize = 0;
                #(
                    let #locals = <#types as #crate_::value::Shape>::from_bits(
                        &bits.slice(offset, <#types as #crate_::value::Shape>::WIDTH));
                    offset = offset.checked_add(<#types as #crate_::value::Shape>::WIDTH)
                        .expect("Shape offset overflow");
                )*
                let _ = offset;
                Self #decoded
            }

            fn make_view(id: #crate_::signal::SignalId, offset: #crate_::value::ShapeSize) -> Self::View {
                let mut offset = offset;
                #(
                    let #locals = <#types as #crate_::value::Shape>::make_view(id, offset);
                    offset = offset.checked_add(<#types as #crate_::value::Shape>::WIDTH)
                        .expect("Shape offset overflow");
                )*
                let _ = (id, offset);
                #view_ident #views
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
