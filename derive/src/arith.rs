use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Path, Result, Type, parse_quote};

#[derive(FromDeriveInput)]
#[darling(attributes(arith))]
struct Options {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn derive(input: TokenStream) -> TokenStream {
    derive_(input).unwrap_or_else(|error| error.to_compile_error())
}

/// Builds a binary arithmetic method: unpacks both operands to the inner
/// shape, delegates, repacks the result.
fn binary(name: &str, op: &str, ty: &Type, crate_: &Path) -> TokenStream {
    let name = format_ident!("{name}");
    let op = format_ident!("{op}");
    quote! {
        fn #name<D: #crate_::domain::Domain>(
            lhs: #crate_::value::Value<D, Self>,
            rhs: #crate_::value::Value<D, Self>,
        ) -> #crate_::value::Value<D, Self> {
            let lhs = <#ty as #crate_::ops::Bits>::unpack(
                <Self as #crate_::ops::Bits>::pack(lhs));
            let rhs = <#ty as #crate_::ops::Bits>::unpack(
                <Self as #crate_::ops::Bits>::pack(rhs));
            let result = <#ty as #crate_::ops::Arith>::#op(lhs, rhs);
            <Self as #crate_::ops::Bits>::unpack(
                <#ty as #crate_::ops::Bits>::pack(result))
        }
    }
}

/// Builds a comparison method returning `bool` from the inner shape.
fn comparison(name: &str, op: &str, ty: &Type, crate_: &Path) -> TokenStream {
    let name = format_ident!("{name}");
    let op = format_ident!("{op}");
    quote! {
        fn #name<D: #crate_::domain::Domain>(
            lhs: #crate_::value::Value<D, Self>,
            rhs: #crate_::value::Value<D, Self>,
        ) -> #crate_::value::Value<D, bool> {
            let lhs = <#ty as #crate_::ops::Bits>::unpack(
                <Self as #crate_::ops::Bits>::pack(lhs));
            let rhs = <#ty as #crate_::ops::Bits>::unpack(
                <Self as #crate_::ops::Bits>::pack(rhs));
            <#ty as #crate_::ops::BitCmp>::#op(lhs, rhs)
        }
    }
}

/// Builds a shift method: unpacks, delegates the shift, repacks.
fn shift(name: &str, op: &str, ty: &Type, crate_: &Path) -> TokenStream {
    let name = format_ident!("{name}");
    let op = format_ident!("{op}");
    quote! {
        fn #name<D: #crate_::domain::Domain>(
            value: #crate_::value::Value<D, Self>,
            amount: usize,
        ) -> #crate_::value::Value<D, Self> {
            let value = <#ty as #crate_::ops::Bits>::unpack(
                <Self as #crate_::ops::Bits>::pack(value));
            let result = <#ty as #crate_::ops::BitShift>::#op(value, amount);
            <Self as #crate_::ops::Bits>::unpack(
                <#ty as #crate_::ops::Bits>::pack(result))
        }
    }
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
            "Arith requires exactly one field",
        ));
    }
    let ty = &data.fields.iter().next().unwrap().ty;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    generics.make_where_clause().predicates.push(parse_quote! {
        #ty: #crate_::ops::Arith + #crate_::ops::BitCmp + #crate_::ops::BitShift
    });
    generics.make_where_clause().predicates.push(parse_quote! {
        #ident #original_ty_generics: #crate_::ops::Bits<
            Packed = <#ty as #crate_::ops::Bits>::Packed
        >
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let add = binary("add", "add", ty, crate_);
    let sub = binary("sub", "sub", ty, crate_);
    let mul = binary("mul", "mul", ty, crate_);
    let gt = comparison("gt", "gt", ty, crate_);
    let ge = comparison("ge", "ge", ty, crate_);
    let lt = comparison("lt", "lt", ty, crate_);
    let le = comparison("le", "le", ty, crate_);
    let shl = shift("shl", "shl", ty, crate_);
    let shr = shift("shr", "shr", ty, crate_);
    Ok(quote! {
        impl #impl_generics #crate_::ops::Arith for #ident #ty_generics #where_clause {
            #add

            #sub

            #mul

            fn from_u128<D: #crate_::domain::Domain>(value: u128) -> #crate_::value::Value<D, Self> {
                let result = <#ty as #crate_::ops::Arith>::from_u128(value);
                <Self as #crate_::ops::Bits>::unpack(
                    <#ty as #crate_::ops::Bits>::pack(result))
            }
        }

        impl #impl_generics #crate_::ops::BitCmp for #ident #ty_generics #where_clause {
            #gt

            #ge

            #lt

            #le
        }

        impl #impl_generics #crate_::ops::BitShift for #ident #ty_generics #where_clause {
            #shl

            #shr
        }
    })
}
