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
    let mut declarations = Vec::new();
    let mut constructors = Vec::new();
    let mut bindings = Vec::new();
    for (index, field) in fields.named.iter().enumerate() {
        let name = field.ident.as_ref().unwrap();
        let label = name.to_string();
        let ty = &field.ty;
        let action = match ty {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .filter(|segment| segment.ident == "Action"),
            _ => None,
        };
        if let Some(segment) = action {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return Err(syn::Error::new_spanned(ty, "expected Action<In, Out>"));
            };
            let arguments: Vec<_> = arguments.args.iter().collect();
            let [GenericArgument::Type(in_ty), GenericArgument::Type(out_ty)] =
                arguments.as_slice()
            else {
                return Err(syn::Error::new_spanned(ty, "expected Action<In, Out>"));
            };
            for ty in [in_ty, out_ty] {
                generics.make_where_clause().predicates.push(parse_quote! {
                    #ty: #crate_::value::Shape
                });
            }
            declarations.push(quote! {
                #crate_::air::InterfaceMember::Method(cx.action_signature::<#in_ty, #out_ty>(#label))
            });
            constructors
                .push(quote! { #name: #crate_::module::Action::from_interface(handle, #label) });
            bindings.push(quote! {
                {
                    let #crate_::air::InterfaceMember::Method(declaration) = &ty.members[#index] else {
                        panic!("expected method declaration");
                    };
                    #crate_::air::InterfaceBinding::Method {
                        declaration,
                        method: cx.action_ref(self.#name.handle()),
                    }
                }
            });
        } else {
            generics.make_where_clause().predicates.push(parse_quote! {
                #ty: #crate_::module::Interface
            });
            declarations.push(quote! {
                #crate_::air::InterfaceMember::Interface {
                    name: cx.alloc_str(#label),
                    ty: cx.iface::<#ty>(),
                }
            });
            constructors.push(quote! {
                #name: <#ty as #crate_::module::Interface>::from_handle(#crate_::__::subinterface(handle, #label))
            });
            bindings.push(quote! {
                #crate_::air::InterfaceBinding::Interface {
                    declaration: &ty.members[#index],
                    implementation: <#ty as #crate_::module::Interface>::air_bind(&self.#name, cx),
                }
            });
        }
    }
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
        impl #impl_generics #crate_::module::Interface for #ident #ty_generics #where_clause {
                    fn air_interface<'dsn>(cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::Interface<'dsn> {
                        let members = [#(#declarations),*];
                        let members = cx.alloc_slice(members);
                        cx.alloc(#crate_::air::Interface {
                            name: cx.alloc_str(stringify!(#ident)),
                            members,
                        })
                    }

                    fn from_handle(handle: #crate_::__::InterfaceHandle) -> Self {
                        Self { #(#constructors),* }
                    }

                    fn air_bind<'dsn>(&self, cx: &mut #crate_::__::Context<'dsn>) -> &'dsn #crate_::air::InterfaceImpl<'dsn> {
                        let ty = cx.iface::<Self>();
                        let members = [#(#bindings),*];
                        let members = cx.alloc_slice(members);
                        cx.alloc(#crate_::air::InterfaceImpl { ty, members })
                    }
                }

        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#methods)*
        }
    })
}
