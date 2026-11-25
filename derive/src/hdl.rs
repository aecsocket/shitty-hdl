use syn::visit::{self, Visit};
use syn::{token::Semi, visit_mut::visit_expr_mut};

use darling::FromMeta;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use std::iter::{once, repeat_n};
use std::mem::replace;
use syn::{
    BinOp, Block, Expr, ExprForLoop, ExprIf, ExprMatch, ExprWhile, Item, ItemFn, Pat, PatWild,
    Path, RangeLimits, Result, Stmt, Token,
    parse::{Parse, Parser},
    parse_quote, parse2,
    punctuated::Punctuated,
    token::{Async, Colon, Else, Move, Underscore},
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
    let crate_ = &options.crate_;
    // An `#[hdl] async fn` is a helper awaited inside a rule or method body:
    // its suspensions extend the caller's state machine, so the body is
    // rewritten as if it were an async body.
    let async_depth = usize::from(function.sig.asyncness.is_some());
    rewrite(&mut function.block, crate_, true, async_depth)?;
    if async_depth > 0 {
        function.block.stmts.insert(
            0,
            parse_quote! {
                #[allow(unused_variables)]
                let __rule_cx = #crate_::module::AsyncCx::assume();
            },
        );
    }
    function
        .attrs
        .push(parse_quote! { #[allow(clippy::future_not_send)] });
    Ok(quote! { #function })
}

pub fn rewrite(
    block: &mut Block,
    crate_: &Path,
    sync_control: bool,
    async_depth: usize,
) -> Result<()> {
    let mut rewriter = Rewriter {
        crate_,
        errors: Vec::new(),
        async_depth,
        elab_depth: 0,
        sync_control,
        in_helper: async_depth > 0,
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
    elab_depth: usize,
    sync_control: bool,
    /// The scope is an `#[hdl] async fn` (or a nested async closure inside
    /// one): Rust jumps complete the helper's future, so the body-level jump
    /// rewrites must not run and `return` may not cross a lowered loop.
    in_helper: bool,
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

fn is_elab(expr: &Expr) -> bool {
    match expr {
        Expr::Call(call) if call.args.len() == 1 => matches!(
            &*call.func,
            Expr::Path(path)
                if path.path.segments.last().is_some_and(|segment| segment.ident == "elab")
        ),
        Expr::Paren(paren) => is_elab(&paren.expr),
        Expr::Group(group) => is_elab(&group.expr),
        _ => false,
    }
}

/// A range whose bounds are both integer literals and whose index is ignored
/// has no other source of type information; pick the counter width from the
/// literal suffixes.
fn literal_range_width(range: &syn::ExprRange) -> Option<usize> {
    fn literal(expr: &Expr) -> Option<&syn::LitInt> {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Int(int) => Some(int),
                _ => None,
            },
            Expr::Paren(paren) => literal(&paren.expr),
            Expr::Group(group) => literal(&group.expr),
            _ => None,
        }
    }
    fn width(suffix: &str) -> Option<usize> {
        Some(match suffix {
            "u8" | "i8" => 8,
            "u16" | "i16" => 16,
            "u32" | "i32" => 32,
            "u64" | "i64" => 64,
            "u128" | "i128" => 128,
            "usize" | "isize" => usize::BITS as usize,
            _ => return None,
        })
    }
    let start = literal(range.start.as_deref()?)?;
    let end = literal(range.end.as_deref()?)?;
    match (start.suffix(), end.suffix()) {
        ("", "") => Some(32),
        ("", suffix) | (suffix, "") => width(suffix),
        (a, b) if a == b => width(a),
        _ => None,
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
            if let Some(pat) = closure.inputs.get_mut(1) {
                // A tuple pattern destructures the method's argument via
                // `unpack`: the argument is one `Value<D, In>`, and the
                // values form is a tuple of one value per input port.
                let tuple_pat: Option<Pat> = match pat {
                    Pat::Type(typed) => match &*typed.pat {
                        Pat::Tuple(tuple) if !tuple.elems.is_empty() => {
                            Some(parse_quote! { #tuple })
                        }
                        _ => None,
                    },
                    Pat::Tuple(tuple) if !tuple.elems.is_empty() => Some(parse_quote! { #tuple }),
                    _ => None,
                };
                if let Some(tuple_pat) = tuple_pat {
                    let unpacked: Stmt = parse_quote! {
                        let #tuple_pat = #crate_::value::unpack(__args);
                    };
                    *pat = parse_quote! { __args };
                    match &mut *closure.body {
                        Expr::Block(block) => block.block.stmts.insert(0, unpacked),
                        body => {
                            *body = parse_quote! {{ #unpacked #body }};
                        }
                    }
                }
            }
            if let Some(pat) = closure.inputs.get_mut(1)
                && matches!(pat, Pat::Tuple(pat) if pat.elems.is_empty())
            {
                *pat = parse_quote! { _ };
            }
            finish_body(closure);
        }
    }

    fn match_arms(&mut self, m: &ExprMatch) -> Result<Vec<(Option<Expr>, Expr)>> {
        let crate_ = self.crate_;
        let mut elements: Vec<Expr> = match &*m.expr {
            Expr::Tuple(tuple) => tuple.elems.iter().cloned().collect(),
            other => vec![other.clone()],
        };
        for element in &mut elements {
            self.visit_expr_mut(element);
        }

        let mut arms: Vec<(Option<Expr>, Expr)> = Vec::new();
        for arm in &m.arms {
            // syn 3 folds the `if` guard into the pattern.
            let (arm_pat, guard) = match &arm.pat {
                Pat::Guard(guarded) => (guarded.pat.as_ref(), Some(guarded.guard.as_ref())),
                pat => (pat, None),
            };
            let alternatives = pattern_alternatives(arm_pat)?;
            let mut conditions: Vec<Expr> = Vec::new();
            for alternative in &alternatives {
                let element_patterns = element_patterns(alternative, elements.len(), arm_pat)?;
                let mut parts: Vec<Expr> = Vec::new();
                for (element, pat) in elements.iter().zip(&element_patterns) {
                    if let Some(part) = element_equality(crate_, element, pat) {
                        parts.push(part);
                    }
                }
                if let Some(condition) = parts
                    .into_iter()
                    .reduce(|lhs, rhs| parse_quote! { #lhs | #rhs })
                {
                    conditions.push(condition);
                }
            }
            let mut condition = conditions
                .into_iter()
                .reduce(|lhs, rhs| parse_quote! { #lhs & #rhs });
            if let Some(guard) = guard {
                let mut guard = (*guard).clone();
                self.visit_expr_mut(&mut guard);
                condition = Some(match condition {
                    Some(patterns) => parse_quote! { #patterns & #guard },
                    None => guard,
                });
            }
            let mut body = match &*arm.body {
                Expr::Block(block) => parse_quote! { #block },
                other => parse_quote! {{ #other }},
            };
            if let Expr::Block(block) = &mut body {
                self.branch(&mut block.block);
            }
            arms.push((condition, body));
        }
        let last_is_catch_all = match m.arms.last() {
            Some(arm) => {
                let (arm_pat, guard) = match &arm.pat {
                    Pat::Guard(guarded) => (guarded.pat.as_ref(), Some(guarded.guard.as_ref())),
                    pat => (pat, None),
                };
                arm_is_catch_all(arm_pat, guard)
            }
            None => false,
        };
        if !last_is_catch_all {
            return Err(syn::Error::new_spanned(
                &m.arms.last().unwrap().pat,
                "hardware `match` must be exhaustive: add a `_` arm",
            ));
        }
        Ok(arms)
    }

    fn async_match(&mut self, m: &ExprMatch) -> Result<Expr> {
        let arms = self.match_arms(m)?;
        if arms.len() == 1 && arms[0].0.is_none() {
            return Ok(arms.into_iter().next().unwrap().1);
        }
        let mut chain: Option<Expr> = None;
        let arm_count = arms.len();
        for (reverse_index, (condition, body)) in arms.into_iter().rev().enumerate() {
            let is_outermost = reverse_index == arm_count - 1;
            let body = parse_quote! { async move || #body };
            chain = Some(match (condition, chain.take()) {
                (None, None) => body,
                (None, Some(tail)) if is_outermost => parse_quote! {
                    __rule_cx.if_(true, #body).else_(#tail).await
                },
                (None, Some(tail)) => parse_quote! {
                    async move || __rule_cx.if_(true, #body).else_(#tail).await
                },
                (Some(condition), Some(tail)) if is_outermost => parse_quote! {
                    __rule_cx.if_(#condition, #body).else_(#tail).await
                },
                (Some(condition), Some(tail)) => parse_quote! {
                    async move || __rule_cx.if_(#condition, #body).else_(#tail).await
                },
                (Some(condition), None) => parse_quote! {
                    __rule_cx.if_(#condition, #body).await
                },
            });
        }
        Ok(chain.expect("match arms are non-empty"))
    }

    fn sync_match(&mut self, m: &ExprMatch) -> Result<Expr> {
        let arms = self.match_arms(m)?;
        if arms.len() == 1 && arms[0].0.is_none() {
            return Ok(arms.into_iter().next().unwrap().1);
        }
        let crate_ = self.crate_;
        let mut chain: Option<Expr> = None;
        let arm_count = arms.len();
        for (reverse_index, (condition, body)) in arms.into_iter().rev().enumerate() {
            let is_outermost = reverse_index == arm_count - 1;
            let body = parse_quote! { move || #body };
            chain = Some(match (condition, chain.take()) {
                (None, None) => body,
                (None, Some(tail)) if is_outermost => parse_quote! {
                    #crate_::__::if_(true, #body, #tail)
                },
                (None, Some(tail)) => parse_quote! {
                    move || #crate_::__::if_(true, #body, #tail)
                },
                (Some(condition), Some(tail)) if is_outermost => parse_quote! {
                    #crate_::__::if_(#condition, #body, #tail)
                },
                (Some(condition), Some(tail)) => parse_quote! {
                    move || #crate_::__::if_(#condition, #body, #tail)
                },
                (Some(condition), None) => parse_quote! {
                    #crate_::__::if_(#condition, #body, move || ())
                },
            });
        }
        Ok(chain.expect("match arms are non-empty"))
    }
}

fn parse_pattern(tokens: TokenStream) -> Result<Pat> {
    struct Pattern(Pat);
    impl Parse for Pattern {
        fn parse(input: syn::parse::ParseStream) -> Result<Self> {
            Ok(Self(Pat::parse_multi(input)?))
        }
    }
    let Pattern(pat) = parse2::<Pattern>(tokens)?;
    Ok(pat)
}

/// Expands vine's `sym!`: one pattern becomes the diagonal pair, two
/// patterns become both orderings.
fn expand_sym(mac: &syn::Macro) -> Result<Vec<Pat>> {
    let mut parts: Vec<TokenStream> = vec![TokenStream::new()];
    for token in mac.tokens.clone() {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            parts.push(TokenStream::new());
        } else {
            parts.last_mut().unwrap().extend(once(token));
        }
    }
    match parts.as_slice() {
        [one] => {
            let pat = parse_pattern(one.clone())?;
            Ok(vec![parse_quote! { (#pat, #pat) }])
        }
        [a, b] => {
            let a = parse_pattern(a.clone())?;
            let b = parse_pattern(b.clone())?;
            Ok(vec![parse_quote! { (#a, #b) | (#b, #a) }])
        }
        _ => Err(syn::Error::new_spanned(
            mac,
            "`sym!` takes one pattern, or two comma-separated patterns",
        )),
    }
}

fn pattern_alternatives(pat: &Pat) -> Result<Vec<Pat>> {
    match pat {
        Pat::Or(or) => or
            .cases
            .iter()
            .map(pattern_alternatives)
            .collect::<Result<Vec<_>>>()
            .map(|alternatives| alternatives.concat()),
        Pat::Macro(macro_)
            if macro_
                .mac
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "sym") =>
        {
            let alternatives = expand_sym(&macro_.mac)?;
            alternatives
                .iter()
                .map(pattern_alternatives)
                .collect::<Result<Vec<_>>>()
                .map(|alternatives| alternatives.concat())
        }
        Pat::Wild(_)
        | Pat::Path(_)
        | Pat::Ident(_)
        | Pat::Lit(_)
        | Pat::Tuple(_)
        | Pat::Type(_) => Ok(vec![pat.clone()]),
        _ => Err(syn::Error::new_spanned(
            pat,
            "unsupported pattern in hardware `match`: use variant paths, literals, `_`, or-patterns, or tuples of those",
        )),
    }
}

fn element_patterns(alternative: &Pat, count: usize, span: &Pat) -> Result<Vec<Pat>> {
    match alternative {
        Pat::Wild(_) => Ok(repeat_n(
            Pat::Wild(PatWild {
                attrs: Vec::new(),
                underscore_token: Underscore::default(),
            }),
            count,
        )
        .collect()),
        Pat::Tuple(tuple) => {
            if tuple.elems.len() != count {
                return Err(syn::Error::new_spanned(
                    span,
                    "hardware `match` pattern does not cover every scrutinee element",
                ));
            }
            Ok(tuple.elems.iter().cloned().collect())
        }
        Pat::Path(_) | Pat::Ident(_) | Pat::Lit(_) => {
            if count == 1 {
                Ok(vec![alternative.clone()])
            } else {
                Err(syn::Error::new_spanned(
                    span,
                    "hardware `match` pattern does not cover every scrutinee element",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            span,
            "unsupported pattern in hardware `match`",
        )),
    }
}

/// Structural equality against a pattern; wildcards match anything.
fn element_equality(crate_: &Path, element: &Expr, pat: &Pat) -> Option<Expr> {
    match pat {
        Pat::Or(or) => or
            .cases
            .iter()
            .filter_map(|case| element_equality(crate_, element, case))
            .reduce(|lhs, rhs| parse_quote! { #lhs | #rhs }),
        Pat::Path(path) => Some(parse_quote! {
            #crate_::ops::BitEqExt::eq(#element, #crate_::value::IntoValue::into_value(#path))
        }),
        Pat::Ident(ident) => Some(parse_quote! {
            #crate_::ops::BitEqExt::eq(#element, #crate_::value::IntoValue::into_value(#ident))
        }),
        Pat::Lit(lit) => {
            let lit = &lit.lit;
            Some(parse_quote! {
                #crate_::ops::BitEqExt::eq(#element, #crate_::value::IntoValue::into_value(#lit))
            })
        }
        _ => None,
    }
}

fn arm_is_catch_all(pat: &Pat, guard: Option<&Expr>) -> bool {
    if guard.is_some() {
        return false;
    }
    match pattern_alternatives(pat) {
        Ok(alternatives) => alternatives.iter().all(|alternative| match alternative {
            Pat::Wild(_) => true,
            Pat::Tuple(tuple) => tuple.elems.iter().all(|pat| matches!(pat, Pat::Wild(_))),
            _ => false,
        }),
        Err(_) => false,
    }
}

fn match_patterns_lowerable(m: &ExprMatch) -> bool {
    m.arms.iter().all(|arm| {
        let (arm_pat, guard) = match &arm.pat {
            Pat::Guard(guarded) => (guarded.pat.as_ref(), Some(guarded.guard.as_ref())),
            pat => (pat, None),
        };
        pattern_alternatives(arm_pat).is_ok() && guard.is_none_or(match_patterns_lowerable_expr)
    })
}

const fn match_patterns_lowerable_expr(expr: &Expr) -> bool {
    !matches!(expr, Expr::Match(_) | Expr::Macro(_) | Expr::Closure(_))
}

impl Rewriter<'_> {
    fn elab_block(&mut self, block: &mut Block) {
        self.elab_depth += 1;
        self.visit_block_mut(block);
        self.elab_depth -= 1;
    }

    fn elab_expr(&mut self, expr: &mut Expr) {
        self.elab_depth += 1;
        self.visit_expr_mut(expr);
        self.elab_depth -= 1;
    }

    fn branch(&mut self, block: &mut Block) {
        self.check_control_flow(block, false);
        self.visit_block_mut(block);
    }

    fn loop_body(&mut self, block: &mut Block) {
        // The body owns its iteration transition: a call is appended on every
        // path that reaches the end, and `break`/`continue` stop elaboration
        // at a state write. The body is driven as one future per state, so the
        // statements after a jump are spliced into the paths that do not jump.
        block.stmts.push(parse_quote! { __rule_cx.loop_next(); });
        self.splice_loop_control(block);
        self.rewrite_loop_control(block);
        // An early return from inside a lowered loop cannot target the
        // enclosing method, so reject anything that still crosses the loop.
        // In helper bodies the `return` rewrite is skipped, so the check sees
        // the original jump.
        self.check_control_flow(block, true);
        self.visit_block_mut(block);
    }

    fn check_control_flow(&mut self, block: &mut Block, in_loop: bool) {
        BranchControlFlow {
            errors: &mut self.errors,
            in_loop,
            elab_loops: 0,
            reject_loop_return: self.in_helper,
        }
        .visit_block_mut(block);
    }

    /// Moves the statements following a `break`/`continue`/`return` into the
    /// paths that do not jump, so every jump ends its block. The body is driven
    /// as one future per state, so a Rust jump cannot cross a segment; the jump
    /// itself is rewritten to a state write by [`Self::rewrite_loop_control`].
    fn splice_loop_control(&mut self, block: &mut Block) {
        for stmt in &mut block.stmts {
            match stmt {
                Stmt::Expr(expr, _) => self.splice_loop_control_expr(expr),
                Stmt::Local(local) => {
                    if let Some(init) = &mut local.init {
                        self.splice_loop_control_expr(&mut init.expr);
                    }
                }
                _ => {}
            }
        }
        let mut index = 0;
        while index < block.stmts.len() {
            if !stmt_may_terminate(&block.stmts[index]) {
                index += 1;
                continue;
            }
            let rest: Vec<Stmt> = block.stmts.drain(index + 1..).collect();
            self.splice_rest(&mut block.stmts[index], &rest);
            index += 1;
        }
    }

    fn splice_loop_control_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::If(if_) => {
                self.splice_loop_control(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.splice_loop_control_expr(branch);
                }
            }
            Expr::Match(match_) => {
                for arm in &mut match_.arms {
                    self.splice_loop_control_expr(&mut arm.body);
                }
            }
            Expr::Block(block) => self.splice_loop_control(&mut block.block),
            Expr::Paren(paren) => self.splice_loop_control_expr(&mut paren.expr),
            Expr::Group(group) => self.splice_loop_control_expr(&mut group.expr),
            _ => {}
        }
    }

    fn splice_rest(&mut self, stmt: &mut Stmt, rest: &[Stmt]) {
        let Stmt::Expr(expr, _) = stmt else {
            return;
        };
        match expr {
            Expr::If(if_) => self.splice_rest_if(if_, rest),
            Expr::Match(match_) => {
                for arm in &mut match_.arms {
                    if expr_terminates(&arm.body) {
                        continue;
                    }
                    append_rest(&mut arm.body, rest);
                    self.splice_loop_control_expr(&mut arm.body);
                }
            }
            _ => {}
        }
    }

    fn splice_rest_if(&mut self, if_: &mut ExprIf, rest: &[Stmt]) {
        if !block_terminates(&if_.then_branch) {
            append_statements(&mut if_.then_branch, rest);
            self.splice_loop_control(&mut if_.then_branch);
        }
        match &mut if_.else_branch {
            None => {
                if_.else_branch = Some((Else::default(), Box::new(parse_quote! {{ #(#rest)* }})));
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.splice_loop_control_expr(branch);
                }
            }
            Some((_, branch)) => match branch.as_mut() {
                Expr::If(nested) => self.splice_rest_if(nested, rest),
                Expr::Block(block) => {
                    if block_terminates(&block.block) {
                        return;
                    }
                    append_statements(&mut block.block, rest);
                    self.splice_loop_control(&mut block.block);
                }
                other => {
                    *other = parse_quote! {{ #other #(#rest)* }};
                    self.splice_loop_control_expr(other);
                }
            },
        }
    }

    /// Replaces the jumps left at block tails: `break` becomes a state write
    /// for the innermost loop's exit, `continue` becomes the loop's iteration
    /// transition, and `return` completes the enclosing body.
    fn rewrite_loop_control(&mut self, block: &mut Block) {
        for stmt in &mut block.stmts {
            match stmt {
                Stmt::Expr(Expr::Break(break_), _) => {
                    self.reject_labelled_jump(break_.label.as_ref());
                    *stmt = parse_quote! { __rule_cx.break_(); };
                }
                Stmt::Expr(Expr::Continue(continue_), _) => {
                    self.reject_labelled_jump(continue_.label.as_ref());
                    *stmt = parse_quote! { __rule_cx.loop_next(); };
                }
                Stmt::Expr(Expr::Return(_), _) if !self.in_helper => {
                    let old = replace(stmt, parse_quote! { (); });
                    let Stmt::Expr(Expr::Return(ret), _) = old else {
                        unreachable!()
                    };
                    *stmt = return_statement(ret);
                }
                Stmt::Expr(expr, _) => self.rewrite_loop_control_expr(expr),
                Stmt::Local(local) => {
                    if let Some(init) = &mut local.init {
                        self.rewrite_loop_control_expr(&mut init.expr);
                    }
                }
                _ => {}
            }
        }
    }

    fn rewrite_loop_control_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Break(break_) => {
                self.reject_labelled_jump(break_.label.as_ref());
                *expr = parse_quote! {{ __rule_cx.break_(); }};
            }
            Expr::Continue(continue_) => {
                self.reject_labelled_jump(continue_.label.as_ref());
                *expr = parse_quote! {{ __rule_cx.loop_next(); }};
            }
            Expr::Return(_) if !self.in_helper => {
                let old = replace(expr, parse_quote! { () });
                let Expr::Return(ret) = old else {
                    unreachable!()
                };
                *expr = return_expression(ret);
            }
            Expr::If(if_) => {
                self.rewrite_loop_control(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.rewrite_loop_control_expr(branch);
                }
            }
            Expr::Match(match_) => {
                for arm in &mut match_.arms {
                    self.rewrite_loop_control_expr(&mut arm.body);
                }
            }
            Expr::Block(block) => self.rewrite_loop_control(&mut block.block),
            Expr::Paren(paren) => self.rewrite_loop_control_expr(&mut paren.expr),
            Expr::Group(group) => self.rewrite_loop_control_expr(&mut group.expr),
            _ => {}
        }
    }

    fn reject_labelled_jump(&mut self, label: Option<&syn::Lifetime>) {
        if let Some(label) = label {
            self.errors.push(syn::Error::new_spanned(
                label,
                "labeled `break`/`continue` is not supported in a hardware loop",
            ));
        }
    }

    fn async_if(&mut self, if_: &mut ExprIf) -> Expr {
        self.visit_expr_mut(&mut if_.cond);
        self.branch(&mut if_.then_branch);
        let crate_ = self.crate_;
        let else_: syn::Expr = match if_.else_branch.take() {
            Some((_, branch)) => match *branch {
                Expr::If(else_if) if !has_let(&else_if.cond) && !is_elab(&else_if.cond) => {
                    let mut else_if = else_if;
                    let nested = self.async_if(&mut else_if);
                    parse_quote! { async move || #nested }
                }
                mut branch => {
                    BranchControlFlow {
                        errors: &mut self.errors,
                        in_loop: false,
                        elab_loops: 0,
                        reject_loop_return: self.in_helper,
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

    fn sync_if(&mut self, if_: &mut ExprIf) -> Expr {
        self.visit_expr_mut(&mut if_.cond);
        self.branch(&mut if_.then_branch);
        let crate_ = self.crate_;
        let else_: syn::Expr = match if_.else_branch.take() {
            Some((_, branch)) => match *branch {
                Expr::If(mut else_if) if !has_let(&else_if.cond) && !is_elab(&else_if.cond) => {
                    let nested = self.sync_if(&mut else_if);
                    parse_quote! { move || #nested }
                }
                mut branch => {
                    BranchControlFlow {
                        errors: &mut self.errors,
                        in_loop: false,
                        elab_loops: 0,
                        reject_loop_return: self.in_helper,
                    }
                    .visit_expr_mut(&mut branch);
                    self.visit_expr_mut(&mut branch);
                    parse_quote! { move || #branch }
                }
            },
            None => parse_quote! { move || () },
        };
        let cond = &if_.cond;
        let then = &if_.then_branch;
        let attrs = &if_.attrs;
        parse_quote! {
            #(#attrs)* {
                #crate_::__::if_(
                    #crate_::value::IntoValue::into_value(#cond),
                    move || #then,
                    #else_,
                )
            }
        }
    }

    fn async_while(&mut self, while_: &mut ExprWhile) -> Result<Expr> {
        if while_.label.is_some() {
            return Err(syn::Error::new_spanned(
                &while_.label,
                "labeled hardware `while` loops are not supported",
            ));
        }
        if has_let(&while_.cond) {
            return Err(syn::Error::new_spanned(
                &while_.cond,
                "hardware `while` conditions must be a boolean value; `while let` is not supported",
            ));
        }
        self.visit_expr_mut(&mut while_.cond);
        self.loop_body(&mut while_.body);
        let crate_ = self.crate_;
        let cond = &while_.cond;
        let body = &while_.body;
        let attrs = &while_.attrs;
        Ok(parse_quote! {
            #(#attrs)* {
                __rule_cx.while_(
                    #crate_::value::IntoValue::into_value(#cond),
                    async move || #body,
                )
                .await
            }
        })
    }

    fn async_for(&mut self, for_: &mut ExprForLoop) -> Result<Expr> {
        if for_.label.is_some() {
            return Err(syn::Error::new_spanned(
                &for_.label,
                "labeled hardware `for` loops are not supported",
            ));
        }
        let Expr::Range(range) = &mut *for_.expr else {
            unreachable!("async_for only handles range expressions")
        };
        let (Some(start), Some(end)) = (&range.start, &range.end) else {
            return Err(syn::Error::new_spanned(
                &*for_.expr,
                "hardware `for` requires a bounded range such as `start..end`",
            ));
        };
        let inclusive = matches!(range.limits, RangeLimits::Closed(_));
        let literal_width = literal_range_width(range);
        let mut start = (**start).clone();
        let mut end = (**end).clone();
        self.visit_expr_mut(&mut start);
        self.visit_expr_mut(&mut end);
        self.loop_body(&mut for_.body);
        let pat = &for_.pat;
        let body = &for_.body;
        let attrs = &for_.attrs;
        if matches!(&*for_.pat, Pat::Wild(_))
            && let Some(width) = literal_width
        {
            let width = proc_macro2::Literal::usize_unsuffixed(width);
            let method: syn::Ident = if inclusive {
                parse_quote! { repeat_inclusive }
            } else {
                parse_quote! { repeat }
            };
            return Ok(parse_quote! {
                #(#attrs)* {
                    __rule_cx.#method::<#width, _, _>(
                        #start,
                        #end,
                        async move || #body,
                    )
                    .await
                }
            });
        }
        let method: syn::Ident = if inclusive {
            parse_quote! { for_inclusive }
        } else {
            parse_quote! { for_ }
        };
        Ok(parse_quote! {
            #(#attrs)* {
                __rule_cx.#method(
                    #start,
                    #end,
                    async move |__for_value| {
                        let #pat = __for_value;
                        #body
                    },
                )
                .await
            }
        })
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
        // Helper bodies complete via Rust jumps, so the body-level rewrite
        // (which retargets `return` at the enclosing body) must not run.
        if self.async_depth > 0 && self.elab_depth == 0 && !self.in_helper {
            self.splice_loop_control(block);
            self.rewrite_loop_control(block);
        }
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
        if is_elab(expr) {
            return;
        }
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

            Expr::Match(match_) if is_elab(&match_.expr) => {
                for arm in &mut match_.arms {
                    self.elab_expr(&mut arm.body);
                }
                return;
            }
            Expr::Match(match_) if self.async_depth > 0 => {
                if !match_patterns_lowerable(match_) {
                    eprintln!("MATCH NOT LOWERABLE");
                }
                match self.async_match(match_) {
                    Ok(generated) => {
                        *expr = generated;
                        return;
                    }
                    Err(error) => {
                        self.errors.push(error);
                        return;
                    }
                }
            }
            Expr::Match(match_) if self.sync_control => match self.sync_match(match_) {
                Ok(generated) => {
                    *expr = generated;
                    return;
                }
                Err(error) => {
                    self.errors.push(error);
                    return;
                }
            },
            Expr::While(while_) if is_elab(&while_.cond) => {
                self.elab_block(&mut while_.body);
                return;
            }
            Expr::While(while_) if self.async_depth > 0 => match self.async_while(while_) {
                Ok(generated) => {
                    *expr = generated;
                    return;
                }
                Err(error) => {
                    self.errors.push(error);
                    return;
                }
            },
            Expr::ForLoop(for_) if is_elab(&for_.expr) => {
                self.elab_block(&mut for_.body);
                return;
            }
            Expr::ForLoop(for_)
                if self.async_depth > 0 && matches!(&*for_.expr, Expr::Range(_)) =>
            {
                match self.async_for(for_) {
                    Ok(generated) => {
                        *expr = generated;
                        return;
                    }
                    Err(error) => {
                        self.errors.push(error);
                        return;
                    }
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
            if is_elab(&if_.cond) {
                self.elab_block(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.elab_expr(branch);
                }
            } else if has_let(&if_.cond) {
                self.visit_block_mut(&mut if_.then_branch);
                if let Some((_, branch)) = &mut if_.else_branch {
                    self.visit_expr_mut(branch);
                }
            } else if self.async_depth > 0 {
                *expr = self.async_if(if_);
            } else if self.sync_control {
                *expr = self.sync_if(if_);
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

/// Whether the block's tail always leaves its block through `return`,
/// `break`, or `continue` (possibly through every path of an `if`/`match`).
fn block_terminates(block: &Block) -> bool {
    block.stmts.last().is_some_and(stmt_terminates)
}

fn stmt_terminates(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Expr(expr, _) if expr_terminates(expr))
}

fn expr_terminates(expr: &Expr) -> bool {
    match expr {
        Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => true,
        Expr::Block(block) => block_terminates(&block.block),
        Expr::If(if_) => {
            block_terminates(&if_.then_branch)
                && if_
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, branch)| expr_terminates(branch))
        }
        Expr::Match(match_) => {
            !match_.arms.is_empty() && match_.arms.iter().all(|arm| expr_terminates(&arm.body))
        }
        Expr::Paren(paren) => expr_terminates(&paren.expr),
        Expr::Group(group) => expr_terminates(&group.expr),
        _ => false,
    }
}

/// Whether the block's tail can leave through a jump on some path.
fn block_may_terminate(block: &Block) -> bool {
    block.stmts.last().is_some_and(stmt_may_terminate)
}

fn stmt_may_terminate(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Expr(expr, _) if expr_may_terminate(expr))
}

fn expr_may_terminate(expr: &Expr) -> bool {
    match expr {
        Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => true,
        Expr::Block(block) => block_may_terminate(&block.block),
        Expr::If(if_) => {
            block_may_terminate(&if_.then_branch)
                || if_
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, branch)| expr_may_terminate(branch))
        }
        Expr::Match(match_) => match_.arms.iter().any(|arm| expr_may_terminate(&arm.body)),
        Expr::Paren(paren) => expr_may_terminate(&paren.expr),
        Expr::Group(group) => expr_may_terminate(&group.expr),
        _ => false,
    }
}

fn append_rest(body: &mut Expr, rest: &[Stmt]) {
    if rest.is_empty() {
        return;
    }
    let current = replace(body, parse_quote! { () });
    *body = parse_quote! {{ #current; #(#rest)* }};
}

/// Turns the body of a `define_as` closure into an explicit completion: the
/// closure returns `()`, and the tail value (or an explicit tail `return`)
/// becomes a `finish` call. A `return` anywhere else is completed by the jump
/// rewrite, so branches never need to unify with the tail's type.
fn finish_body(closure: &mut syn::ExprClosure) {
    let body = replace(&mut *closure.body, parse_quote! { () });
    *closure.body = match body {
        Expr::Return(ret) => {
            let statement = finish_statement(ret);
            parse_quote! {{ #statement }}
        }
        Expr::Block(mut block) => {
            finish_block(&mut block.block);
            Expr::Block(block)
        }
        other => parse_quote! {{ __rule_cx.finish(#other); }},
    };
}

fn finish_block(block: &mut Block) {
    let Some(last) = block.stmts.len().checked_sub(1) else {
        block.stmts.push(parse_quote! { __rule_cx.finish_unit(); });
        return;
    };
    if matches!(&block.stmts[last], Stmt::Expr(Expr::Return(_), _)) {
        let Stmt::Expr(Expr::Return(ret), _) = block.stmts.pop().unwrap() else {
            unreachable!()
        };
        block.stmts.push(finish_statement(ret));
        return;
    }
    if !matches!(&block.stmts[last], Stmt::Expr(_, None)) {
        block.stmts.push(parse_quote! { __rule_cx.finish_unit(); });
        return;
    }
    // A trailing block-like expression that jumps cannot be the body's value:
    // keep it as a statement so the jump rewrite can see inside, and finish
    // after it (the continuation is spliced into the paths that do not jump).
    let statement_only = matches!(
        &block.stmts[last],
        Stmt::Expr(Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_), None)
    ) || matches!(
        &block.stmts[last],
        Stmt::Expr(
            Expr::If(_) | Expr::Match(_) | Expr::Block(_),
            None
        ) if matches!(&block.stmts[last], Stmt::Expr(expr, None) if expr_contains_return(expr))
    );
    if statement_only {
        block.stmts.push(parse_quote! { __rule_cx.finish_unit(); });
        return;
    }
    let Stmt::Expr(expr, _) = &mut block.stmts[last] else {
        unreachable!()
    };
    let expr = replace(expr, parse_quote! { () });
    block.stmts[last] = parse_quote! { __rule_cx.finish(#expr); };
}

/// Finds a `return` in a block-like tail expression. Nested loops own their
/// jumps (their own lowering rewrites them), so this does not descend into
/// them, and closures are separate bodies.
fn expr_contains_return(expr: &Expr) -> bool {
    struct FindReturn(bool);

    impl<'ast> Visit<'ast> for FindReturn {
        fn visit_expr(&mut self, expr: &'ast Expr) {
            if matches!(expr, Expr::Return(_)) {
                self.0 = true;
            }
            visit::visit_expr(self, expr);
        }

        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}

        fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}

        fn visit_expr_while(&mut self, _: &'ast syn::ExprWhile) {}

        fn visit_expr_for_loop(&mut self, _: &'ast syn::ExprForLoop) {}

        fn visit_expr_loop(&mut self, _: &'ast syn::ExprLoop) {}
    }

    let mut finder = FindReturn(false);
    finder.visit_expr(expr);
    finder.0
}

/// Appends statements to a block, terminating a trailing expression first.
fn append_statements(block: &mut Block, rest: &[Stmt]) {
    if rest.is_empty() {
        return;
    }
    if let Some(Stmt::Expr(_, semi @ None)) = block.stmts.last_mut() {
        *semi = Some(Semi::default());
    }
    block.stmts.extend(rest.iter().cloned());
}

fn finish_statement(ret: syn::ExprReturn) -> Stmt {
    match ret.expr {
        Some(value) => parse_quote! { __rule_cx.finish(#value); },
        None => parse_quote! { __rule_cx.finish_unit(); },
    }
}

fn return_statement(ret: syn::ExprReturn) -> Stmt {
    match ret.expr {
        Some(value) => parse_quote! { __rule_cx.return_(#value); },
        None => parse_quote! { __rule_cx.return_unit(); },
    }
}

fn return_expression(ret: syn::ExprReturn) -> Expr {
    match ret.expr {
        Some(value) => parse_quote! {{ __rule_cx.return_(#value); }},
        None => parse_quote! {{ __rule_cx.return_unit(); }},
    }
}

struct BranchControlFlow<'a> {
    errors: &'a mut Vec<syn::Error>,
    in_loop: bool,
    elab_loops: usize,
    reject_loop_return: bool,
}

impl VisitMut for BranchControlFlow<'_> {
    fn visit_item_mut(&mut self, _item: &mut Item) {}

    fn visit_expr_closure_mut(&mut self, _closure: &mut syn::ExprClosure) {}

    fn visit_expr_async_mut(&mut self, _expr: &mut syn::ExprAsync) {}

    // `loop` is not lowered, so its jumps belong to Rust.
    fn visit_expr_loop_mut(&mut self, _loop_: &mut syn::ExprLoop) {}

    // A nested hardware loop owns its own jumps; its body is checked when it is
    // lowered. An `elab`-marked loop is plain Rust, so its jumps stay legal but
    // a `return` inside it still cannot cross the enclosing state machine.
    fn visit_expr_while_mut(&mut self, while_: &mut ExprWhile) {
        if is_elab(&while_.cond) {
            self.elab_loops += 1;
            visit_mut::visit_expr_while_mut(self, while_);
            self.elab_loops -= 1;
        }
    }

    fn visit_expr_for_loop_mut(&mut self, for_: &mut ExprForLoop) {
        if is_elab(&for_.expr) {
            self.elab_loops += 1;
            visit_mut::visit_expr_for_loop_mut(self, for_);
            self.elab_loops -= 1;
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let jump = matches!(expr, Expr::Break(_) | Expr::Continue(_));
        if jump && self.elab_loops > 0 {
            visit_expr_mut(self, expr);
            return;
        }
        if matches!(
            expr,
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) | Expr::Try(_)
        ) {
            let message = if self.in_loop {
                match expr {
                    Expr::Break(_) | Expr::Continue(_) => {
                        "`break`/`continue` must be a statement in the loop body or the tail of an `if`/`match` arm"
                    }
                    Expr::Return(_) if self.reject_loop_return => {
                        "`return` is not supported inside a hardware loop of an `#[hdl]` async function; restructure the loop or make it a rule/method body"
                    }
                    Expr::Return(_) => {
                        "`return` must be a statement in the body, not part of an expression"
                    }
                    _ => "`?` is not supported",
                }
            } else {
                "unsupported branch control flow: moving this expression into an async branch would change its control-flow target"
            };
            self.errors.push(syn::Error::new_spanned(&*expr, message));
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
