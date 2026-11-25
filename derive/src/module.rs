use darling::{FromMeta, util::Flag};
use proc_macro2::{Span, TokenStream};
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
    if let Some(asyncness) = function.sig.asyncness {
        return Err(syn::Error::new_spanned(
            asyncness,
            "async modules are not supported; module elaboration must be synchronous",
        ));
    }
    // Item names do not get local-variable hygiene, so also avoid argument names.
    let mut elaborate_name = "__module_elaborate".to_owned();
    while function.sig.inputs.iter().any(|arg| {
        matches!(arg,
        FnArg::Typed(arg) if matches!(arg.pat.as_ref(),
            Pat::Ident(pat) if pat.ident == elaborate_name.as_str()))
    }) {
        elaborate_name.push('_');
    }
    let elaborate = syn::Ident::new(&elaborate_name, Span::mixed_site());
    let scope = syn::Ident::new("__module_scope", Span::mixed_site());

    // make:
    // - function params for instantiating the module
    // - field definitions for the resulting port `struct`
    let InOuts {
        call_params,
        elaborate_params,
        port_fields,
        allocations,
        arguments,
        initializers,
    } = make_in_outs(&options, function.sig.inputs.iter(), &mut errors);

    // rewrite control flow (`if`, `else`, `while` etc.) in the body,
    // and save that as the statements for the elaborate fn;
    // the real fn with name `ident` will just make the port.
    let elaborate_stmts = make_elaborate_stmts(&options, function.block.stmts, &mut errors);

    let builder = if options.builder.is_present() {
        quote! { #[#crate_::__::bon::builder(crate = #crate_::__::bon)] }
    } else {
        quote! {}
    };

    if !errors.is_empty() {
        let mut error = errors.remove(0);
        for next in errors {
            error.combine(next);
        }
        return Err(error);
    }
    // Infer lifetimes: explicitly supplying late-bound lifetimes is invalid.
    let call_generics: Vec<_> = function
        .sig
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(param) => {
                let ident = &param.ident;
                Some(quote! { #ident })
            }
            syn::GenericParam::Const(param) => {
                let ident = &param.ident;
                Some(quote! { #ident })
            }
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    let turbofish = if call_generics.is_empty() {
        quote! {}
    } else {
        quote! { ::<#(#call_generics),*> }
    };
    let vis = function.vis;
    let ident = function.sig.ident;
    Ok(quote! {

        #builder
        #vis fn #ident #impl_generics (
            #(#call_params,)*
        ) -> #ident #ty_generics #where_clause {
            fn #elaborate #impl_generics (
                #(#elaborate_params,)*
            ) #where_clause {
                #(#elaborate_stmts)*
            }
            let #scope = #crate_::__::enter_module(<#ident #ty_generics as #crate_::__::Module>::NAME);
            #(#allocations)*
            #elaborate #turbofish (#(#arguments,)*);
            #ident { #(#initializers,)* }
        }

        #[allow(non_camel_case_types, reason = "macro-generated")]
        #vis struct #ident #impl_generics #where_clause {
            #(#port_fields,)*
        }

        impl #impl_generics #crate_::__::Module for #ident #ty_generics #where_clause {
            const NAME: &'static str = ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#ident));
        }
    })
}

struct InOuts {
    call_params: Vec<TokenStream>,
    elaborate_params: Vec<TokenStream>,
    port_fields: Vec<TokenStream>,
    allocations: Vec<TokenStream>,
    arguments: Vec<TokenStream>,
    initializers: Vec<TokenStream>,
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
        Domain,
        // e.g. `fn x<T>(_phantom: PhantomData<T>)`
        Phantom,
    }

    let crate_ = &options.crate_;
    let mut call_params = Vec::new();
    let mut elaborate_params = Vec::new();
    let mut port_fields = Vec::new();
    let mut allocations = Vec::new();
    let mut arguments = Vec::new();
    let mut initializers = Vec::new();
    let inputs: Vec<_> = inputs.collect();
    let mut field_names: Vec<_> = inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(input) => match input.pat.as_ref() {
                Pat::Ident(pat) => Some(pat.ident.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    for (idx, input) in inputs.into_iter().enumerate() {
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
        let is_domain = last_path_ident.is_some_and(|id| id == "Domain");
        let kind = if is_phantom_data {
            InputKind::Phantom
        } else if is_signal {
            InputKind::Signal
        } else if is_domain {
            InputKind::Domain
        } else {
            InputKind::Arg
        };

        let ty = &input.ty;
        match kind {
            InputKind::Arg => {
                // args like `capacity: u32` are passed directly
                let pat = &input.pat;
                match pat.as_ref() {
                    Pat::Ident(pat) if pat.subpat.is_none() && pat.by_ref.is_none() => {
                        let ident = &pat.ident;
                        let mut argument = input.clone();
                        if let Pat::Ident(pat) = argument.pat.as_mut() {
                            pat.mutability = None;
                        }
                        call_params.push(quote! { #argument });
                        arguments.push(quote! { #ident });
                    }
                    Pat::Wild(_) => {
                        let ident = format_ident!("__module_arg_{idx}", span = Span::mixed_site());
                        call_params.push(quote! { #ident: #ty });
                        arguments.push(quote! { #ident });
                    }
                    _ => errors.push(syn::Error::new_spanned(
                        pat,
                        "module arguments must have a simple name or wildcard pattern",
                    )),
                }
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
                let inner = format_ident!("__module_inner_{idx}", span = Span::mixed_site());
                let outer = format_ident!("__module_outer_{idx}", span = Span::mixed_site());
                allocations.push(quote! {
                    let (#inner, #outer) = <#ty as #crate_::__::SignalFlow>::new_port(stringify!(#ident));
                });
                arguments.push(quote! { #inner });
                initializers.push(quote! { #ident: #outer });
                let ty = quote! { <#ty as #crate_::__::SignalFlow>::Inverse };
                port_fields.push(quote! { pub #ident: #ty });
            }
            InputKind::Domain => {
                arguments.push(quote! { <#ty as ::core::default::Default>::default() });
                // Keep labels used only by injected domains represented in
                // generic module types, without returning runtime handles.
                let mut name = format!("__module_domain_{idx}");
                while field_names.contains(&name) {
                    name.push('_');
                }
                field_names.push(name.clone());
                let ident = syn::Ident::new(&name, Span::mixed_site());
                port_fields.push(quote! { #ident: ::core::marker::PhantomData<#ty> });
                initializers.push(quote! { #ident: ::core::marker::PhantomData });
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
                arguments.push(quote! { ::core::marker::PhantomData });
                initializers.push(quote! { #ident: ::core::marker::PhantomData });
            }
        }
    }

    InOuts {
        call_params,
        elaborate_params,
        port_fields,
        allocations,
        arguments,
        initializers,
    }
}

fn make_elaborate_stmts(
    options: &Options,
    mut stmts: Vec<Stmt>,
    errors: &mut Vec<syn::Error>,
) -> Vec<Stmt> {
    let mut rewriter = MakeElaborate { options, errors };
    for stmt in &mut stmts {
        rewriter.visit_stmt_mut(stmt);
        if let Stmt::Expr(_, semi) = stmt {
            *semi = Some(Default::default());
        }
    }
    stmts
}

struct MakeElaborate<'a> {
    options: &'a Options,
    errors: &'a mut Vec<syn::Error>,
}

// Inspect original branches before rewriting adds closures. Explicit closures
// and items retain their own control-flow targets and must not be rejected.
struct BranchControlFlow<'a> {
    errors: &'a mut Vec<syn::Error>,
}

impl VisitMut for BranchControlFlow<'_> {
    fn visit_expr_closure_mut(&mut self, _closure: &mut syn::ExprClosure) {}

    fn visit_item_mut(&mut self, _item: &mut syn::Item) {}

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let keyword = match expr {
            Expr::Return(_) => Some("return"),
            Expr::Break(_) => Some("break"),
            Expr::Continue(_) => Some("continue"),
            _ => None,
        };
        if let Some(keyword) = keyword {
            self.errors.push(syn::Error::new_spanned(
                &*expr,
                format!("unsupported branch control flow: `{keyword}` inside a hardware if branch would change meaning when moved into a closure"),
            ));
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

impl VisitMut for MakeElaborate<'_> {
    fn visit_item_mut(&mut self, _item: &mut syn::Item) {}

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        let len = block.stmts.len();
        for (index, stmt) in block.stmts.iter_mut().enumerate() {
            self.visit_stmt_mut(stmt);
            // Rewritten statement expressions may now return Expr<()>. Only
            // discard non-tail values; branch tails can be value-producing.
            if index + 1 < len
                && let Stmt::Expr(_, semi) = stmt
            {
                *semi = Some(Default::default());
            }
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let Expr::If(if_) = expr else {
            syn::visit_mut::visit_expr_mut(self, expr);
            let args = match expr {
                Expr::Call(call)
                    if matches!(call.func.as_ref(), Expr::Path(path)
                        if path.path.segments.last().is_some_and(|segment| segment.ident == "comb")) =>
                {
                    &mut call.args
                }
                Expr::MethodCall(call) if call.method == "on" => &mut call.args,
                _ => return,
            };
            // These callbacks require unit, unlike arbitrary value closures.
            // Their bodies have already been visited; only discard the result.
            for arg in args {
                if let Expr::Closure(closure) = arg {
                    match closure.body.as_mut() {
                        Expr::Block(block) => {
                            if let Some(Stmt::Expr(_, semi)) = block.block.stmts.last_mut() {
                                *semi = Some(Default::default());
                            }
                        }
                        body => *body = parse_quote! { { #body; } },
                    }
                }
            }
            return;
        };

        self.visit_expr_mut(&mut if_.cond);
        BranchControlFlow {
            errors: self.errors,
        }
        .visit_block_mut(&mut if_.then_branch);
        self.visit_block_mut(&mut if_.then_branch);

        let crate_ = &self.options.crate_;
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let mut chain = quote! { #crate_::__::if_(#cond, || #then) };

        // Walk the chain before rewriting its `else if` nodes into calls.
        let mut has_else = false;
        let mut next = &mut if_.else_branch;
        while let Some((_, branch)) = next {
            match branch.as_mut() {
                Expr::If(else_if) => {
                    self.visit_expr_mut(&mut else_if.cond);
                    BranchControlFlow {
                        errors: self.errors,
                    }
                    .visit_block_mut(&mut else_if.then_branch);
                    self.visit_block_mut(&mut else_if.then_branch);
                    let cond = &else_if.cond;
                    let then = &else_if.then_branch;
                    chain = quote! { #chain.else_if(#cond, || #then) };
                    next = &mut else_if.else_branch;
                }
                branch => {
                    BranchControlFlow {
                        errors: self.errors,
                    }
                    .visit_expr_mut(branch);
                    self.visit_expr_mut(branch);
                    chain = quote! { #chain.else_(|| #branch) };
                    has_else = true;
                    break;
                }
            }
        }

        *expr = if has_else {
            parse_quote! { { #chain } }
        } else {
            // Without an else, discard the unfinished builder and yield unit.
            parse_quote! { { #chain; } }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_parameters_are_injected_without_constructor_arguments() {
        let output = derive_(
            quote! {},
            quote! {
                fn example<L: DomainLabel>(
                    mut first: Domain<L>,
                    second: shitty_hdl::module::Domain<Sync>,
                    capacity: u8,
                    output: Out<bool>,
                ) {}
            },
        )
        .unwrap();
        let file = syn::parse2::<syn::File>(output).unwrap();
        let syn::Item::Fn(constructor) = &file.items[0] else {
            panic!("expected constructor")
        };
        assert_eq!(constructor.sig.inputs.len(), 1);
        assert!(
            quote! { #constructor }
                .to_string()
                .contains("Default > :: default")
        );
        let syn::Item::Struct(ports) = &file.items[1] else {
            panic!("expected port struct")
        };
        assert_eq!(ports.fields.len(), 3);
        assert!(
            quote! { #ports }
                .to_string()
                .contains("PhantomData < Domain < L > >")
        );
    }

    #[test]
    fn domain_marker_names_do_not_collide_with_ports() {
        let output = derive_(
            quote! {},
            quote! {
                fn example(domain: Domain<Sync>, __module_domain_0: In<bool>) {}
            },
        )
        .unwrap();
        let file = syn::parse2::<syn::File>(output).unwrap();
        let syn::Item::Struct(ports) = &file.items[1] else {
            panic!("expected port struct")
        };
        let names: Vec<_> = ports
            .fields
            .iter()
            .map(|field| field.ident.as_ref().unwrap().to_string())
            .collect();
        assert_eq!(names, ["__module_domain_0_", "__module_domain_0"]);
    }

    #[test]
    fn rejects_escaping_branch_control_flow() {
        for body in [
            quote! { comb(|| { if stop { return; } output <<= u!(7); }); },
            quote! { loop { if stop { break; } } },
            quote! { loop { if stop {} else if other { continue; } } },
            quote! { loop { if stop {} else { break; } } },
            quote! { 'outer: loop { if stop { break 'outer; } } },
            quote! { if stop { let local = || { if other { return; } }; } },
        ] {
            let error = derive_(quote! {}, quote! { fn example() { #body } }).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("unsupported branch control flow")
            );
        }
    }

    #[test]
    fn accumulates_branch_control_flow_errors() {
        let error = derive_(
            quote! {},
            quote! {
                fn example() {
                    if stop { return; } else if other { break; } else { continue; }
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.into_iter().count(), 3);
    }

    #[test]
    fn permits_local_control_flow_and_returns_outside_branches() {
        derive_(
            quote! {},
            quote! {
                fn example() {
                    if stop {
                        let local = || { return 1; };
                        let local_loop = || { loop { break; } };
                        fn local_item() { return; }
                    }
                    let value = || { return if stop { 1 } else { 2 }; };
                    return;
                }
            },
        )
        .unwrap();
    }
}
