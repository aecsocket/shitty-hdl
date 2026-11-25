use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    BinOp, Error, Expr, FnArg, ItemFn, Pat, Result, Stmt, parse_quote, punctuated::Punctuated,
    token::Comma,
};

pub fn derive(_attr: TokenStream, input_: TokenStream) -> TokenStream {
    let mut errors = Vec::new();
    let crate_ = quote! { shitty_hdl };
    let mut function = match syn::parse2::<ItemFn>(input_) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error(),
    };

    let (impl_generics, ty_generics, where_clause) = function.sig.generics.split_for_impl();

    // make field definitions for the resulting port `struct`
    let port_fields = make_port_fields(function.sig.inputs.clone(), &mut errors);

    // rewrite control flow (`if`, `else`, `while` etc.) in the body
    let stmts = make_fn_stmts(function.block.stmts, &mut errors);
    function.block.stmts = stmts;

    let input_ = quote! { #function };
    let errors = errors.iter().map(syn::Error::to_compile_error);
    let vis = function.vis;
    let ident = function.sig.ident;
    quote! {
        #input_
        #(#errors)*

        #[allow(non_camel_case_types, reason = "macro-generated")]
        #vis struct #ident #ty_generics #where_clause {
            #(#port_fields,)*
        }

        impl #impl_generics #crate_::__::Module for #ident #ty_generics #where_clause {}
    }
}

fn make_port_fields(
    inputs: Punctuated<FnArg, Comma>,
    errors: &mut Vec<syn::Error>,
) -> Vec<TokenStream> {
    let crate_ = quote! { shitty_hdl };

    let mut port_fields = Vec::new();
    for input in inputs {
        let input = match input {
            FnArg::Receiver(_) => {
                errors.push(syn::Error::new_spanned(
                    input,
                    "`self` is not allowed for module functions",
                ));
                continue;
            }
            FnArg::Typed(ty) => ty,
        };

        let ident = match input.pat.as_ref() {
            Pat::Ident(pat) if pat.subpat.is_none() => &pat.ident,
            _ => {
                errors.push(syn::Error::new_spanned(
                    &input.pat,
                    "module inputs must have a simple name",
                ));
                continue;
            }
        };

        let is_phantom_data = matches!(
            input.ty.as_ref(),
            syn::Type::Path(path)
                if path.qself.is_none()
                && path.path.segments.last()
                    .is_some_and(|seg| seg.ident == "PhantomData")
        );

        let field = if is_phantom_data {
            // `PhantomData` gets used in a struct directly
            let ty = &input.ty;
            quote! { pub #ident: #ty }
        } else {
            // this should be an `In` or `Out`,
            // so the struct should take the inverse flow
            let ty = &input.ty;
            let ty = quote! { <#ty as #crate_::__::SignalFlow>::Inverse };
            quote! { pub #ident: #ty }
        };

        port_fields.push(field);
    }
    port_fields
}

fn make_fn_stmts(original: Vec<Stmt>, errors: &mut Vec<syn::Error>) -> Vec<Stmt> {
    let crate_ = quote! { shitty_hdl };

    let mut stmts = Vec::new();
    for stmt in original {
        let Stmt::Expr(expr, semi) = stmt else {
            stmts.push(stmt);
            continue;
        };

        let stmt = match expr {
            Expr::If(if_) => {
                // `if x { then }` -> `if_(x, || then)`
                let cond = match translate_cond(*if_.cond) {
                    Ok(cond) => cond,
                    Err(err) => {
                        errors.push(err);
                        continue;
                    }
                };
                let then = if_.then_branch;
                parse_quote! {
                    #crate_::__::if_(#cond, || #then);
                }
            }
            _ => Stmt::Expr(expr, semi),
        };
        stmts.push(stmt);
    }
    stmts
}

fn translate_cond(expr: Expr) -> Result<Expr> {
    let crate_ = quote! { shitty_hdl };

    let Expr::Binary(bin) = &expr else {
        return Ok(expr);
    };
    let lhs = &bin.left;
    let rhs = &bin.right;

    match bin.op {
        BinOp::Eq(_) => {
            // `a == b` -> `eq(a, b)`
            Ok(parse_quote! { #crate_::__::eq(#lhs, #rhs) })
        }
        BinOp::Ne(_) => {
            // `a != b` -> `ne(a, b)`
            Ok(parse_quote! { #crate_::__::ne(#lhs, #rhs) })
        }
        BinOp::Gt(_) => {
            // `a > b` -> `gt(a, b)`
            Ok(parse_quote! { #crate_::__::gt(#lhs, #rhs) })
        }
        BinOp::Ge(_) => {
            // `a >= b` -> `ge(a, b)`
            Ok(parse_quote! { #crate_::__::ge(#lhs, #rhs) })
        }
        BinOp::Lt(_) => {
            // `a < b` -> `lt(a, b)`
            Ok(parse_quote! { #crate_::__::lt(#lhs, #rhs) })
        }
        BinOp::Le(_) => {
            // `a <= b` -> `le(a, b)`
            Ok(parse_quote! { #crate_::__::le(#lhs, #rhs) })
        }
        _ => Err(Error::new_spanned(expr, "unsupported binary operation")),
    }
}
