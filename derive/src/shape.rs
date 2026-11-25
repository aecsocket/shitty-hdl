#![allow(
    clippy::absolute_paths,
    reason = "darling expands field types into multi-segment paths"
)]
use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::primitive::usize;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Path, Result, ext::IdentExt, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(shape))]
#[allow(
    clippy::absolute_paths,
    reason = "darling expands field types into its own paths"
)]
struct Options {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
    width: Option<usize>,
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
            let explicit = data
                .variants
                .iter()
                .any(|variant| variant.discriminant.is_some());
            let output = if explicit || options.width.is_some() {
                packed_enum(&input, data, &options)?
            } else {
                named_enum(&input, data, crate_)
            };
            return syn::parse2(output);
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
    let width = data.fields.iter().map(|field| {
        let ty = &field.ty;
        quote! { <#ty as #crate_::ops::Bits>::WIDTH }
    });
    let width = quote! { (0usize #(+ #width)*) };
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
    }
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: 'static
    });
    let metadata = data.fields.iter().enumerate().map(|(index, field)| {
        let ty = &field.ty;
        let name = field
            .ident
            .as_ref()
            .map_or_else(|| format!("_{index}"), |ident| ident.unraw().to_string());
        quote! { #crate_::air::Field { name: #name, ty: cx.ty::<#ty>() } }
    });
    let values = data.fields.iter().enumerate().map(|(index, field)| {
        let ty = &field.ty;
        let member = field.ident.clone().map_or_else(
            || syn::Member::Unnamed(syn::Index::from(index)),
            syn::Member::Named,
        );
        quote! { <#ty as #crate_::value::Shape>::air_value(&self.#member, cx) }
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fields = data.fields.iter().map(|field| {
        let ty = &field.ty;
        let vis = &field.vis;
        let view_ty = quote! { <#ty as #crate_::value::Shape>::View };
        if let Some(ident) = &field.ident {
            quote! { #vis #ident: #view_ty }
        } else {
            quote! { #vis #view_ty }
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
    let width_assert = options.width.map(|annotated| {
        let generic = !input.generics.params.is_empty();
        if generic {
            quote! {}
        } else {
            quote! {
                const _: () = ::core::assert!(
                    <#ident as #crate_::ops::Bits>::WIDTH <= #annotated,
                    ::core::concat!(
                        ::core::stringify!(#ident),
                        " does not fit in the annotated width"
                    )
                );
                const _: () = ::core::assert!(
                    <#ident as #crate_::ops::Bits>::WIDTH >= #annotated,
                    ::core::concat!(
                        ::core::stringify!(#ident),
                        " is narrower than the annotated width; struct shapes occupy exactly their fields' width - add an explicit padding field"
                    )
                );
            }
        }
    });
    let width_body_assert = options.width.and_then(|annotated| {
        let generic = !input.generics.params.is_empty();
        generic.then(|| {
            quote! {
                const WIDTH_FITS: () = ::core::assert!(
                    <Self as #crate_::ops::Bits>::WIDTH <= #annotated,
                    ::core::concat!(
                        ::core::stringify!(#ident),
                        " does not fit in the annotated width"
                    )
                );
                const WIDTH_EXACT: () = ::core::assert!(
                    <Self as #crate_::ops::Bits>::WIDTH >= #annotated,
                    ::core::concat!(
                        ::core::stringify!(#ident),
                        " is narrower than the annotated width; struct shapes occupy exactly their fields' width - add an explicit padding field"
                    )
                );
            }
        })
    });
    let width_body_ref = options.width.and_then(|_annotated| {
        let generic = !input.generics.params.is_empty();
        generic.then(|| quote! { let () = (Self::WIDTH_FITS, Self::WIDTH_EXACT); })
    });
    Ok(quote! {
        #view

        impl #impl_generics ::core::marker::Copy for #view_ident #ty_generics #where_clause {}

        #[allow(clippy::expl_impl_clone_on_copy)]
        impl #impl_generics ::core::clone::Clone for #view_ident #ty_generics #where_clause {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
            type View = #view_ident #ty_generics;

            fn air_type<'dsn>(cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Type<'dsn> {
                let fields = [#(#metadata,)*];
                let fields = cx.alloc_slice(fields);
                cx.named_type::<Self>(#crate_::air::NamedTypeKind::Struct(fields))
            }

            fn air_value<'dsn>(&self, cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Expr<'dsn> {
                let fields = ::std::vec![#(#values,)*];
                cx.struct_value::<Self>(fields)
            }

            type Values<D: #crate_::domain::Domain> = #crate_::value::Value<D, Self>;

            fn make_values<D: #crate_::domain::Domain>(value: #crate_::value::Value<D, Self>) -> Self::Values<D> { value }

            fn pack_values<D: #crate_::domain::Domain>(values: Self::Values<D>) -> #crate_::value::Value<D, Self> { values }

            fn value_handles<D: #crate_::domain::Domain>(values: Self::Values<D>) -> ::std::vec::Vec<#crate_::__::ExprHandle> { ::std::vec![values.handle()] }
        }

        impl #impl_generics #crate_::ops::Bits for #ident #ty_generics #where_clause {
            #width_body_assert

            const WIDTH: usize = #width;
            type Packed = #crate_::value::Bit<{ #width }>;

            fn pack<D: #crate_::domain::Domain>(this: #crate_::value::Value<D, Self>) -> #crate_::value::Value<D, Self::Packed> {
                #width_body_ref
                #crate_::__::cast::<D, Self, Self::Packed>(#crate_::air::CastKind::Pack, this)
            }

            fn unpack<D: #crate_::domain::Domain>(bits: #crate_::value::Value<D, Self::Packed>) -> #crate_::value::Value<D, Self> {
                #width_body_ref
                #crate_::__::cast::<D, Self::Packed, Self>(#crate_::air::CastKind::Unpack, bits)
            }
        }

        #width_assert
    })
}

/// Emits the original lowering: the enum becomes a named BSV enum type,
/// with variants numbered by BSC.
fn named_enum(input: &DeriveInput, data: &syn::DataEnum, crate_: &Path) -> TokenStream {
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: 'static
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let names = data
        .variants
        .iter()
        .map(|variant| variant.ident.unraw().to_string());
    let arms = data.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let name = ident.unraw().to_string();
        quote! { Self::#ident => cx.enum_value::<Self>(#name) }
    });
    quote! {
        impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
            type View = #crate_::value::ValueView<Self>;

            fn air_type<'dsn>(cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Type<'dsn> {
                let variants = cx.alloc_slice([#(#names,)*]);
                cx.named_type::<Self>(#crate_::air::NamedTypeKind::Enum(variants))
            }

            fn air_value<'dsn>(&self, cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Expr<'dsn> {
                match self { #(#arms,)* }
            }

            type Values<D: #crate_::domain::Domain> = #crate_::value::Value<D, Self>;

            fn make_values<D: #crate_::domain::Domain>(value: #crate_::value::Value<D, Self>) -> Self::Values<D> { value }

            fn pack_values<D: #crate_::domain::Domain>(values: Self::Values<D>) -> #crate_::value::Value<D, Self> { values }

            fn value_handles<D: #crate_::domain::Domain>(values: Self::Values<D>) -> ::std::vec::Vec<#crate_::__::ExprHandle> { ::std::vec![values.handle()] }
        }
    }
}

/// Emits a packed lowering: the enum becomes a `Bit<N>` whose values are
/// the variant discriminants. Requires every discriminant to be a
/// non-negative integer literal; `width` must cover the largest one.
fn packed_enum(
    input: &DeriveInput,
    data: &syn::DataEnum,
    options: &Options,
) -> Result<TokenStream> {
    let crate_ = &options.crate_;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: 'static
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut values = Vec::new();
    let mut current: u128 = 0;
    let mut max: u128 = 0;
    for variant in &data.variants {
        let value = match &variant.discriminant {
            Some((_, Expr::Lit(expr))) => match &expr.lit {
                Lit::Int(int) => int.base10_parse::<u128>()?,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &expr.lit,
                        "enum discriminants must be integer literals",
                    ));
                }
            },
            Some((_, expr)) => {
                return Err(syn::Error::new_spanned(
                    expr,
                    "enum discriminants must be integer literals",
                ));
            }
            None => current,
        };
        values.push((variant.ident.clone(), value));
        max = max.max(value);
        current = value + 1;
    }

    let computed = max.next_power_of_two().ilog2().max(1) as usize;
    let width = options.width.unwrap_or(computed);
    if width < 128 && max >= 1u128 << width {
        let offenders: Vec<_> = values
            .iter()
            .filter(|(_, value)| *value >= 1u128 << width)
            .map(|(name, value)| format!("{name} = {value}"))
            .collect();
        return Err(syn::Error::new_spanned(
            input,
            format!(
                "enum does not fit in {width} bits: {}",
                offenders.join(", ")
            ),
        ));
    }

    let arms = values.iter().map(|(name, value)| {
        quote! {
            Self::#name => {
                #crate_::value::Bit::<#width>::from_u128(#value).air_value(cx)
            }
        }
    });
    Ok(quote! {
        impl #impl_generics #crate_::value::Shape for #ident #ty_generics #where_clause {
            type View = #crate_::value::ValueView<Self>;

            fn air_type<'dsn>(cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Type<'dsn> {
                // Delegate to `Bit<#width>` through the type cache so every
                // use of this enum shares its type arena slot with the
                // packed bits (the elaborator compares types by pointer).
                cx.ty::<#crate_::value::Bit<#width>>()
            }

            fn air_value<'dsn>(&self, cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Expr<'dsn> {
                match self { #(#arms,)* }
            }

            type Values<D: #crate_::domain::Domain> = #crate_::value::Value<D, Self>;

            fn make_values<D: #crate_::domain::Domain>(value: #crate_::value::Value<D, Self>) -> Self::Values<D> { value }

            fn pack_values<D: #crate_::domain::Domain>(values: Self::Values<D>) -> #crate_::value::Value<D, Self> { values }

            fn value_handles<D: #crate_::domain::Domain>(values: Self::Values<D>) -> ::std::vec::Vec<#crate_::__::ExprHandle> { ::std::vec![values.handle()] }
        }

        impl #impl_generics #crate_::ops::Bits for #ident #ty_generics #where_clause {
            const WIDTH: usize = #width;
            type Packed = #crate_::value::Bit<#width>;

            fn pack<D: #crate_::domain::Domain>(this: #crate_::value::Value<D, Self>) -> #crate_::value::Value<D, Self::Packed> {
                #crate_::__::cast::<D, Self, Self::Packed>(#crate_::air::CastKind::Pack, this)
            }

            fn unpack<D: #crate_::domain::Domain>(bits: #crate_::value::Value<D, Self::Packed>) -> #crate_::value::Value<D, Self> {
                #crate_::__::cast::<D, Self::Packed, Self>(#crate_::air::CastKind::Unpack, bits)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_structs() {
        let error = derive_(quote! { union Either { a: u8, b: u16 } }).unwrap_err();
        assert!(error.to_string().contains("only be derived for structs"));
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
            quote! { enum Choice { A, B } },
            quote! { enum Tag { Wire = 1, Graft, Erase } },
            quote! { #[shape(width = 3)] enum Tag { Wire = 1, Graft = 4 } },
        ] {
            let output = derive_(input).unwrap();
            if let Err(error) = syn::parse2::<syn::File>(output.clone()) {
                panic!("expansion parse failed: {error}\n{output}");
            }
        }
    }

    #[test]
    fn rejects_width_overflow() {
        let error = derive_(quote! {
            #[shape(width = 2)]
            enum Tag { Wire = 1, Graft, Erase, ExtVal }
        })
        .unwrap_err();
        assert!(error.to_string().contains("does not fit in 2 bits"));
    }
}
