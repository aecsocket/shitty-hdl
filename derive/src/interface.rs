use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Fields, GenericArgument, Path, PathArguments, Result, Type, parse_quote,
};

#[derive(FromDeriveInput)]
#[darling(attributes(interface))]
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
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &input,
            "Interface requires named fields",
        ));
    };
    let mut generics = input.generics.clone();
    let mut methods = Vec::new();
    for field in &fields.named {
        let Type::Path(path) = &field.ty else {
            continue;
        };
        let Some(segment) = path.path.segments.last() else {
            continue;
        };
        if segment.ident != "Method" {
            continue;
        }
        let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "expected Method<In, Out>",
            ));
        };
        let arguments: Vec<_> = arguments.args.iter().collect();
        let [GenericArgument::Type(in_ty), GenericArgument::Type(out_ty)] = arguments.as_slice()
        else {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "expected Method<In, Out>",
            ));
        };
        for ty in [in_ty, out_ty] {
            generics.make_where_clause().predicates.push(parse_quote! {
                #ty: #crate_::value::Shape
            });
        }
        let inputs: Vec<_> = match in_ty {
            Type::Tuple(tuple) => tuple.elems.iter().collect(),
            ty => vec![ty],
        };
        for ty in &inputs {
            generics.make_where_clause().predicates.push(parse_quote! {
                #ty: #crate_::value::Shape
            });
        }
        let names: Vec<_> = (0..inputs.len())
            .map(|i| format_ident!("arg_{i}"))
            .collect();
        let output = match out_ty {
            Type::Tuple(tuple) if tuple.elems.is_empty() => quote! {},
            ty => quote! { -> #crate_::value::Value<#ty> },
        };
        let name = &field.ident;
        let vis = &field.vis;
        methods.push(quote! {
            #[allow(clippy::future_not_send, clippy::unused_async_trait_impl)]
            #vis async fn #name(&self, #(#names: impl #crate_::value::IntoValue<Out = #inputs>),*) #output {
                let _ = (#(#names,)*);
                ::core::todo!()
            }
        });
    }
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics #crate_::module::Interface for #ident #ty_generics #where_clause {}

        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
