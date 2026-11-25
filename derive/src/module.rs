use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{BinOp, Error, Expr, FnArg, ItemFn, Pat, Result, Stmt, Type, parse_quote};

pub fn derive(_attr: TokenStream, input_: TokenStream) -> TokenStream {
    let mut errors = Vec::new();
    let crate_ = quote! { shitty_hdl };
    let function = match syn::parse2::<ItemFn>(input_) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error(),
    };

    let (impl_generics, ty_generics, where_clause) = function.sig.generics.split_for_impl();
    let asyncness = function.sig.asyncness;

    // make:
    // - function params for instantiating the module
    // - field definitions for the resulting port `struct`
    let InOuts {
        call_params,
        elaborate_params,
        port_fields,
    } = make_in_outs(function.sig.inputs.iter(), &mut errors);

    // rewrite control flow (`if`, `else`, `while` etc.) in the body,
    // and save that as the statements for the elaborate fn;
    // the real fn with name `ident` will just make the port.
    let elaborate_stmts = make_elaborate_stmts(function.block.stmts, &mut errors);

    let errors = errors.iter().map(syn::Error::to_compile_error);
    let vis = function.vis;
    let ident = function.sig.ident;
    quote! {
        #(#errors)*

        #vis fn #ident #ty_generics (
            #(#call_params,)*
        ) #where_clause -> #ident #ty_generics {
            todo!();
        }

        const _: () = {
            #asyncness fn elaborate #ty_generics (
                #(#elaborate_params,)*
            ) #where_clause {
                #(#elaborate_stmts)*
            }
        };

        #[allow(non_camel_case_types, reason = "macro-generated")]
        #vis struct #ident #ty_generics #where_clause {
            #(#port_fields,)*
        }

        impl #impl_generics #crate_::__::Module for #ident #ty_generics #where_clause {}
    }
}

struct InOuts {
    call_params: Vec<TokenStream>,
    elaborate_params: Vec<TokenStream>,
    port_fields: Vec<TokenStream>,
}

fn make_in_outs<'a>(
    inputs: impl Iterator<Item = &'a FnArg>,
    errors: &mut Vec<syn::Error>,
) -> InOuts {
    enum InputKind {
        // e.g. `fn x(capacity: u32)`
        Arg,
        // e.g. `fn x(start: In<bool>)`
        Signal,
        // e.g. `fn x<T>(_phantom: PhantomData<T>)`
        Phantom,
    }

    let crate_ = quote! { shitty_hdl };

    let mut call_params = Vec::new();
    let mut elaborate_params = Vec::new();
    let mut port_fields = Vec::new();
    for (idx, input) in inputs.enumerate() {
        elaborate_params.push(quote! { #input });

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

        let last_path_ident = if let Type::Path(path) = input.ty.as_ref()
            && path.qself.is_none()
        {
            path.path.segments.last().map(|seg| &seg.ident)
        } else {
            None
        };

        let is_phantom_data = last_path_ident.is_some_and(|id| id == "PhantomData");
        let is_in = last_path_ident.is_some_and(|id| id == "In");
        let is_out = last_path_ident.is_some_and(|id| id == "Out");
        let is_signal = is_in || is_out;
        let kind = match (is_phantom_data, is_signal) {
            (true, _) => InputKind::Phantom,
            (false, true) => InputKind::Signal,
            (false, false) => InputKind::Arg,
        };

        let ty = &input.ty;
        match kind {
            InputKind::Arg => {
                // args like `capacity: u32` are passed directly
                let pat = &input.pat;
                call_params.push(quote! { #pat: #ty });
            }
            InputKind::Signal => {
                // for signals like `start: In<bool>`:
                // - not needed to instantiate the module
                // - the *inverse flow* is included in the port struct
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
                let ty = quote! { <#ty as #crate_::__::SignalFlow>::Inverse };
                port_fields.push(quote! { pub #ident: #ty })
            }
            InputKind::Phantom => {
                // `PhantomData` is only in the port struct
                //
                // construct a synthetic ident for this,
                // since users might do like `_: PhantomData<T>`;
                // that has no real ident, and it would be pointless
                // to require callers to specify a real ident.
                let ident = format_ident!("_phantom_{idx}");
                port_fields.push(quote! { #ident: #ty });
            }
        }
    }

    InOuts {
        call_params,
        elaborate_params,
        port_fields,
    }
}

fn make_elaborate_stmts(original: Vec<Stmt>, errors: &mut Vec<syn::Error>) -> Vec<Stmt> {
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
