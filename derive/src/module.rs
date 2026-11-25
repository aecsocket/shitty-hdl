use darling::{FromMeta, util::Flag};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, FnArg, ItemFn, Pat, Result, Stmt, Type, parse_quote, visit_mut::VisitMut};

#[derive(Default, FromMeta)]
#[darling(derive_syn_parse)]
struct Options {
    builder: Flag,
    #[darling(rename = "crate")]
    crate_: Option<syn::Path>,
}

pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_(attr, input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive_(attr: TokenStream, input_: TokenStream) -> Result<TokenStream> {
    let function = syn::parse2::<ItemFn>(input_)?;
    let mut options = syn::parse2::<Options>(attr)?;
    options
        .crate_
        .get_or_insert_with(|| parse_quote! { ::shitty_hdl });
    let crate_ = &options.crate_;
    let mut errors = Vec::new();

    let (impl_generics, ty_generics, where_clause) = function.sig.generics.split_for_impl();
    let asyncness = function.sig.asyncness;

    // make:
    // - function params for instantiating the module
    // - field definitions for the resulting port `struct`
    let InOuts {
        call_params,
        elaborate_params,
        port_fields,
    } = make_in_outs(&options, function.sig.inputs.iter(), &mut errors);

    // rewrite control flow (`if`, `else`, `while` etc.) in the body,
    // and save that as the statements for the elaborate fn;
    // the real fn with name `ident` will just make the port.
    let elaborate_stmts = make_elaborate_stmts(&options, function.block.stmts);

    let builder = if options.builder.is_present() {
        quote! { #[#crate_::__::bon::builder(crate = #crate_::__::bon)] }
    } else {
        quote! {}
    };

    let errors = errors.iter().map(syn::Error::to_compile_error);
    let vis = function.vis;
    let ident = function.sig.ident;
    Ok(quote! {
        #(#errors)*

        #builder
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
    })
}

struct InOuts {
    call_params: Vec<TokenStream>,
    elaborate_params: Vec<TokenStream>,
    port_fields: Vec<TokenStream>,
}

fn make_in_outs<'a>(
    options: &Options,
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

    let crate_ = &options.crate_;
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

fn make_elaborate_stmts(options: &Options, mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut rewriter = MakeElaborate { options };
    for stmt in &mut stmts {
        rewriter.visit_stmt_mut(stmt);
    }
    stmts
}

struct MakeElaborate<'a> {
    options: &'a Options,
}

impl VisitMut for MakeElaborate<'_> {
    fn visit_item_mut(&mut self, _item: &mut syn::Item) {}

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let Expr::If(if_) = expr else {
            syn::visit_mut::visit_expr_mut(self, expr);
            return;
        };

        self.visit_expr_mut(&mut if_.cond);
        self.visit_block_mut(&mut if_.then_branch);

        let crate_ = &self.options.crate_;
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let mut chain = quote! { #crate_::__::if_(#cond, || #then) };

        // Walk the chain before rewriting its `else if` nodes into calls.
        let mut next = &mut if_.else_branch;
        while let Some((_, branch)) = next {
            match branch.as_mut() {
                Expr::If(else_if) => {
                    self.visit_expr_mut(&mut else_if.cond);
                    self.visit_block_mut(&mut else_if.then_branch);
                    let cond = &else_if.cond;
                    let then = &else_if.then_branch;
                    chain = quote! { #chain.else_if(#cond, || #then) };
                    next = &mut else_if.else_branch;
                }
                branch => {
                    self.visit_expr_mut(branch);
                    chain = quote! { #chain.or_else(|| { #branch }) };
                    break;
                }
            }
        }

        *expr = parse_quote! { { #chain; } };
    }
}
