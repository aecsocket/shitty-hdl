use syn::{token::Semi, visit_mut::visit_expr_mut};

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    BinOp, Block, Expr, ExprIf, Item, ItemFn, Pat, Path, Result, Stmt, Token,
    parse::Parser,
    parse_quote,
    punctuated::Punctuated,
    token::{Async, Colon, Else, Move},
    visit_mut,
    visit_mut::VisitMut,
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

pub fn rewrite(block: &mut Block, crate_: &Path) -> Result<()> {
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
    fn prepend_cx(&self, arg: &mut Expr) {
        if let Expr::Closure(closure) = arg {
            let crate_ = self.crate_;
            closure.asyncness = Some(Async::default());
            // Capture hardware by value: registers, outputs, and views are
            // `Copy` handles, so each async closure owns its own copy and
            // writes through it to the same register - two branches may
            // then write the same register without aliasing `&mut`.
            closure.capture = Some(Move::default());
            let cx: syn::Type = parse_quote! { #crate_::module::AsyncCx };
            closure.inputs.insert(
                0,
                syn::Pat::Type(syn::PatType {
                    attrs: Vec::new(),
                    pat: Box::new(parse_quote! { __rule_cx }),
                    colon_token: Colon::default(),
                    ty: Box::new(cx),
                }),
            );
            if let Some(pat) = closure.inputs.get_mut(1)
                && matches!(pat, Pat::Tuple(pat) if pat.elems.is_empty())
            {
                *pat = parse_quote! { _ };
            }
        }
    }

    fn transform_early_returns(&mut self, block: &mut Block) {
        EarlyReturns {
            errors: &mut self.errors,
        }
        .visit_block_mut(block);
        for stmt in &block.stmts {
            if let Stmt::Expr(Expr::Return(_), _) = stmt {
                self.errors.push(syn::Error::new_spanned(
                    stmt,
                    "early return must be the last statement of an `if` block",
                ));
            }
        }
    }

    fn branch(&mut self, block: &mut Block) {
        self.transform_early_returns(block);
        BranchControlFlow {
            errors: &mut self.errors,
        }
        .visit_block_mut(block);
        self.visit_block_mut(block);
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
                    parse_quote! { async move || #nested }
                }
                mut branch => {
                    BranchControlFlow {
                        errors: &mut self.errors,
                    }
                    .visit_expr_mut(&mut branch);
                    self.visit_expr_mut(&mut branch);
                    parse_quote! { async move || #branch }
                }
            },
            None => parse_quote! { async move || {} },
        };
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let attrs = &if_.attrs;
        parse_quote! {
            #(#attrs)* {
                __rule_cx.if_(
                    #crate_::value::IntoValue::into_value(#cond),
                    async move || #then,
                )
                .else_(#else_)
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
        let display = segment.ident == "display" || segment.ident == "displayln";
        if !display && segment.ident != "sim_assert" {
            return;
        }
        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        match parser.parse2(mac.tokens.clone()) {
            Ok(mut args) if display && (!args.is_empty() || segment.ident == "displayln") => {
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
                    "expected displayln!(), display!/displayln!(value), or display!/displayln!(format, arguments...)"
                } else {
                    "expected sim_assert!(condition) or sim_assert!(condition, message)"
                },
            )),
            Err(error) => self.errors.push(error),
        }
    }

    fn visit_item_mut(&mut self, _item: &mut Item) {}

    fn visit_block_mut(&mut self, block: &mut Block) {
        self.transform_early_returns(block);
        let len = block.stmts.len();
        for (index, stmt) in block.stmts.iter_mut().enumerate() {
            let was_if = matches!(stmt, Stmt::Expr(Expr::If(_), _));
            self.visit_stmt_mut(stmt);
            if was_if
                && index + 1 < len
                && let Stmt::Expr(_, semi) = stmt
            {
                *semi = Some(Semi::default());
            }
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Call(call) => {
                if matches!(&*call.func, Expr::Path(path) if
                    path.path.segments.last().is_some_and(|s| s.ident == "define_as"))
                {
                    for arg in &mut call.args {
                        self.prepend_cx(arg);
                    }
                }
            }
            Expr::MethodCall(call) if call.method == "define_as" => {
                for arg in &mut call.args {
                    self.prepend_cx(arg);
                }
            }
            _ => {}
        }
        if is_async_closure(expr) {
            self.async_depth += 1;
            visit_expr_mut(self, expr);
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
                // Plain Rust control flow at module scope; visit the
                // pieces without rewriting the `if` itself.
                self.visit_expr_mut(&mut if_.cond);
                self.visit_block_mut(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.visit_expr_mut(branch);
                }
            }
            return;
        }
        visit_expr_mut(self, expr);
        let Expr::Binary(binary) = expr else {
            return;
        };
        let crate_ = self.crate_;
        let function: Path = match &binary.op {
            BinOp::Eq(_) => parse_quote! { #crate_::ops::BitEqExt::eq },
            BinOp::Ne(_) => parse_quote! { #crate_::ops::BitEqExt::ne },
            BinOp::Gt(_) => parse_quote! { #crate_::ops::BitCmpExt::gt },
            BinOp::Ge(_) => parse_quote! { #crate_::ops::BitCmpExt::ge },
            BinOp::Lt(_) => parse_quote! { #crate_::ops::BitCmpExt::lt },
            BinOp::Le(_) => parse_quote! { #crate_::ops::BitCmpExt::le },
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

/// Desugars early returns: statements following an `if` whose block ends
/// with `return;` move into that `if`'s else branch, so they execute only
/// on the non-early path.
struct EarlyReturns<'a> {
    errors: &'a mut Vec<syn::Error>,
}

impl VisitMut for EarlyReturns<'_> {
    fn visit_block_mut(&mut self, block: &mut Block) {
        visit_mut::visit_block_mut(self, block);
        self.splice(block);
    }
}

impl EarlyReturns<'_> {
    fn splice(&mut self, block: &mut Block) {
        let mut index = 0;
        while index < block.stmts.len() {
            let ends_with_return = matches!(
                &block.stmts[index],
                Stmt::Expr(Expr::If(if_), _)
                    if matches!(
                        if_.then_branch.stmts.last(),
                        Some(Stmt::Expr(Expr::Return(_), _))
                    )
            );
            if !ends_with_return {
                index += 1;
                continue;
            }
            let rest: Vec<Stmt> = block.stmts.drain(index + 1..).collect();
            let Stmt::Expr(Expr::If(if_), semi) = &mut block.stmts[index] else {
                unreachable!()
            };
            if let Some(Stmt::Expr(Expr::Return(ret), _)) = if_.then_branch.stmts.last()
                && let Some(value) = &ret.expr
            {
                self.errors.push(syn::Error::new_spanned(
                    value,
                    "early return with a value is not supported",
                ));
            }
            if_.then_branch.stmts.pop();
            match &mut if_.else_branch {
                Some((_, branch)) => {
                    if let Expr::Block(else_block) = &mut **branch {
                        else_block.block.stmts.extend(rest);
                    } else {
                        self.errors.push(syn::Error::new_spanned(
                            &**branch,
                            "cannot combine an early return with this else branch",
                        ));
                    }
                }
                None => {
                    if_.else_branch =
                        Some((Else::default(), Box::new(parse_quote! { { #(#rest)* } })));
                }
            }
            *semi = Some(Semi::default());
            index += 1;
        }
    }
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
        visit_expr_mut(self, expr);
    }
}

#[cfg(test)]
mod probe_tests {
    #[test]
    fn parses_annotated_pat() {
        let pat: syn::Pat = syn::parse_quote! { __rule_cx };
        assert!(matches!(pat, syn::Pat::Ident(_)), "plain: {pat:?}");
    }
}
