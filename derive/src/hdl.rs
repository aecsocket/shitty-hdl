use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    BinOp, Block, Expr, ExprIf, Item, ItemFn, Path, Result, Stmt, Token, parse::Parser,
    parse_quote, punctuated::Punctuated, visit_mut::VisitMut,
};

#[derive(FromMeta)]
#[darling(derive_syn_parse)]
struct Options {
    #[darling(rename = "crate", default = crate::root)]
    crate_: Path,
}

pub fn derive(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_(attr, input).unwrap_or_else(|error| error.to_compile_error())
}

fn derive_(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let options = syn::parse2::<Options>(attr)?;
    let mut function = syn::parse2::<ItemFn>(input)?;
    rewrite(&mut function.block, &options.crate_)?;
    function
        .attrs
        .push(parse_quote! { #[allow(clippy::future_not_send)] });
    Ok(quote! { #function })
}

pub(crate) fn rewrite(block: &mut Block, crate_: &Path) -> Result<()> {
    let mut rewriter = Rewriter {
        crate_,
        errors: Vec::new(),
        async_depth: 0,
    };
    rewriter.visit_block_mut(block);
    let mut errors = rewriter.errors.into_iter();
    if let Some(mut error) = errors.next() {
        for next in errors {
            error.combine(next);
        }
        return Err(error);
    }
    Ok(())
}

struct Rewriter<'a> {
    crate_: &'a Path,
    errors: Vec<syn::Error>,
    async_depth: usize,
}

fn has_let(expr: &Expr) -> bool {
    match expr {
        Expr::Let(_) => true,
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            has_let(&binary.left) || has_let(&binary.right)
        }
        Expr::Paren(paren) => has_let(&paren.expr),
        Expr::Group(group) => has_let(&group.expr),
        _ => false,
    }
}

const fn is_async_closure(expr: &Expr) -> bool {
    match expr {
        Expr::Async(_) => true,
        Expr::Closure(closure) => closure.asyncness.is_some(),
        _ => false,
    }
}

impl Rewriter<'_> {
    fn branch(&mut self, block: &mut Block) {
        BranchControlFlow {
            errors: &mut self.errors,
        }
        .visit_block_mut(block);
        self.visit_block_mut(block);
    }

    fn hardware_if(&mut self, if_: &mut ExprIf) -> Expr {
        self.visit_expr_mut(&mut if_.cond);
        self.branch(&mut if_.then_branch);
        let crate_ = self.crate_;
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let attrs = &if_.attrs;
        let mut chain = quote! {
            #crate_::module::if_(#crate_::value::IntoValue::into_value(#cond), async || #then)
        };
        let mut next = &mut if_.else_branch;
        while let Some((_, branch)) = next {
            if matches!(branch.as_ref(), Expr::If(else_if) if !has_let(&else_if.cond)) {
                if let Expr::If(else_if) = branch.as_mut() {
                    self.visit_expr_mut(&mut else_if.cond);
                    self.branch(&mut else_if.then_branch);
                    let cond = &else_if.cond;
                    let then = &else_if.then_branch;
                    chain = quote! {
                        #chain.else_if_cond(|| #crate_::value::IntoValue::into_value(#cond)).then(async || #then)
                    };
                    next = &mut else_if.else_branch;
                } else {
                    unreachable!();
                }
            } else {
                BranchControlFlow {
                    errors: &mut self.errors,
                }
                .visit_expr_mut(branch);
                self.visit_expr_mut(branch);
                return parse_quote! { #(#attrs)* { #chain.else_(async || { #branch }) } };
            }
        }
        parse_quote! { #(#attrs)* { ::core::mem::drop(#chain); } }
    }
    fn async_if(&mut self, if_: &mut ExprIf) -> Expr {
        self.visit_expr_mut(&mut if_.cond);
        self.branch(&mut if_.then_branch);
        let crate_ = self.crate_;
        let else_: syn::Expr = match if_.else_branch.take() {
            Some((_, branch)) => match *branch {
                Expr::If(else_if) if !has_let(&else_if.cond) => {
                    let mut else_if = else_if;
                    let nested = self.async_if(&mut else_if);
                    parse_quote! { async || #nested }
                }
                mut branch => {
                    BranchControlFlow {
                        errors: &mut self.errors,
                    }
                    .visit_expr_mut(&mut branch);
                    self.visit_expr_mut(&mut branch);
                    parse_quote! { async || #branch }
                }
            },
            None => parse_quote! { async || {} },
        };
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let attrs = &if_.attrs;
        parse_quote! {
            #(#attrs)* {
                #crate_::module::AsyncIf::new(
                    #crate_::value::IntoValue::into_value(#cond),
                    async || #then,
                    #else_,
                )
                .await
            }
        }
    }
}

impl VisitMut for Rewriter<'_> {
    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        let Some(segment) = mac.path.segments.last() else {
            return;
        };
        let display = segment.ident == "display";
        if !display && segment.ident != "sim_assert" {
            return;
        }
        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        match parser.parse2(mac.tokens.clone()) {
            Ok(mut args) if display && !args.is_empty() => {
                for argument in &mut args {
                    self.visit_expr_mut(argument);
                }
                mac.tokens = quote! { #args };
            }
            Ok(mut args) if !display && (1..=2).contains(&args.len()) => {
                self.visit_expr_mut(args.first_mut().unwrap());
                mac.tokens = quote! { #args };
            }
            Ok(_) => self.errors.push(syn::Error::new_spanned(
                mac,
                if display {
                    "expected display!(value) or display!(format, arguments...)"
                } else {
                    "expected sim_assert!(condition) or sim_assert!(condition, message)"
                },
            )),
            Err(error) => self.errors.push(error),
        }
    }

    fn visit_item_mut(&mut self, _item: &mut Item) {}

    fn visit_block_mut(&mut self, block: &mut Block) {
        let len = block.stmts.len();
        for (index, stmt) in block.stmts.iter_mut().enumerate() {
            let was_if = matches!(stmt, Stmt::Expr(Expr::If(_), _));
            self.visit_stmt_mut(stmt);
            if was_if
                && index + 1 < len
                && let Stmt::Expr(_, semi) = stmt
            {
                *semi = Some(Default::default());
            }
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            Expr::MethodCall(call) if call.method == "define_as" => {
                if call.args.iter().any(is_async_closure) {
                    call.method = syn::Ident::new("define_async_as", call.method.span());
                }
            }
            Expr::Call(call) => {
                if let Expr::Path(path) = &mut *call.func
                    && let Some(segment) = path.path.segments.last_mut()
                    && segment.ident == "define_as"
                    && call.args.iter().any(is_async_closure)
                {
                    segment.ident = syn::Ident::new("define_async_as", segment.ident.span());
                }
            }
            _ => {}
        }
        if is_async_closure(expr) {
            self.async_depth += 1;
            syn::visit_mut::visit_expr_mut(self, expr);
            self.async_depth -= 1;
            return;
        }
        if let Expr::If(if_) = expr {
            if has_let(&if_.cond) {
                self.visit_block_mut(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.visit_expr_mut(branch);
                }
            } else if self.async_depth > 0 {
                *expr = self.async_if(if_);
            } else {
                *expr = self.hardware_if(if_);
            }
            return;
        }
        syn::visit_mut::visit_expr_mut(self, expr);
        let Expr::Binary(binary) = expr else {
            return;
        };
        let crate_ = self.crate_;
        let function: Path = match &binary.op {
            BinOp::Eq(_) => parse_quote! { #crate_::ops::BitCmpExt::eq },
            BinOp::Ne(_) => parse_quote! { #crate_::ops::BitCmpExt::ne },
            BinOp::Gt(_) => parse_quote! { #crate_::ops::ArithCmpExt::gt },
            BinOp::Ge(_) => parse_quote! { #crate_::ops::ArithCmpExt::ge },
            BinOp::Lt(_) => parse_quote! { #crate_::ops::ArithCmpExt::lt },
            BinOp::Le(_) => parse_quote! { #crate_::ops::ArithCmpExt::le },
            _ => return,
        };
        let attrs = &binary.attrs;
        let lhs = &binary.left;
        let rhs = &binary.right;
        *expr = parse_quote! { #(#attrs)* #function(#lhs, #rhs) };
    }
}

struct BranchControlFlow<'a> {
    errors: &'a mut Vec<syn::Error>,
}

impl VisitMut for BranchControlFlow<'_> {
    fn visit_item_mut(&mut self, _item: &mut Item) {}

    fn visit_expr_closure_mut(&mut self, _closure: &mut syn::ExprClosure) {}

    fn visit_expr_async_mut(&mut self, _expr: &mut syn::ExprAsync) {}

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if matches!(
            expr,
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) | Expr::Try(_)
        ) {
            self.errors.push(syn::Error::new_spanned(
                &*expr,
                "unsupported branch control flow: moving this expression into an async branch would change its control-flow target",
            ));
        }
        if matches!(expr, Expr::Assign(_))
            || matches!(expr, Expr::Binary(binary) if matches!(binary.op,
                BinOp::AddAssign(_) | BinOp::SubAssign(_) | BinOp::MulAssign(_)
                | BinOp::DivAssign(_) | BinOp::RemAssign(_) | BinOp::BitXorAssign(_)
                | BinOp::BitAndAssign(_) | BinOp::BitOrAssign(_) | BinOp::ShrAssign(_)
            ))
        {
            self.errors.push(syn::Error::new_spanned(
                &*expr,
                "unsupported assignment in hardware branch: all branches execute during elaboration, so Rust-side mutation is not conditional; use <<= for register writes",
            ));
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}
