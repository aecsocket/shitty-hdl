use std::collections::HashMap;
use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Path, Token};

struct Input {
    crate_: Path,
    format: Option<LitStr>,
    args: Vec<(Option<Ident>, Expr)>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let crate_ = input.parse()?;
        input.parse::<Token![;]>()?;
        if !input.peek(LitStr) {
            let value = input.parse()?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            if !input.is_empty() {
                return Err(input.error("display value accepts no additional arguments"));
            }
            return Ok(Self {
                crate_,
                format: None,
                args: vec![(None, value)],
            });
        }
        let format = input.parse()?;
        let mut args = Vec::new();
        let mut named = false;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let fork = input.fork();
            let name = if fork.parse::<Ident>().is_ok() && fork.peek(Token![=]) {
                let name = input.parse()?;
                input.parse::<Token![=]>()?;
                named = true;
                Some(name)
            } else {
                if named {
                    return Err(input.error("positional arguments must precede named arguments"));
                }
                None
            };
            args.push((name, input.parse()?));
        }
        Ok(Self {
            crate_,
            format: Some(format),
            args,
        })
    }
}

fn name_key(name: &Ident) -> String {
    let name = name.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_owned()
}

fn conversion(spec: &str, span: Span) -> syn::Result<Spec> {
    let width_end = spec.bytes().take_while(u8::is_ascii_digit).count();
    let (width, kind) = spec.split_at(width_end);
    if !width.is_empty() && width.parse::<u32>().is_err() {
        return Err(syn::Error::new(span, "format width is too large"));
    }
    let (hardware_kind, text_kind) = match kind {
        "" => ('d', 'd'),
        "x" => ('x', 'x'),
        "?" => ('h', 'x'),
        "b" => ('b', 'b'),
        "o" => ('o', 'o'),
        _ => {
            return Err(syn::Error::new(
                span,
                "unsupported display format; use an optional literal width followed by x, b, o, ?, or no type",
            ));
        }
    };
    Ok(Spec {
        hardware: format!(
            "%{}{hardware_kind}",
            if width.is_empty() { "0" } else { width }
        ),
        width: width.parse().unwrap_or(0),
        kind: text_kind,
    })
}

struct Spec {
    hardware: String,
    width: usize,
    kind: char,
}

enum Piece {
    Text(String),
    Arg { index: usize, spec: Spec },
}

fn expand(input: Input) -> syn::Result<TokenStream> {
    let Input {
        crate_,
        format,
        args,
    } = input;
    let mut expressions = Vec::new();
    let mut names = HashMap::new();
    let mut positional = Vec::new();
    for (name, expr) in args {
        let index = expressions.len();
        if let Some(name) = name {
            if names.insert(name_key(&name), index).is_some() {
                return Err(syn::Error::new_spanned(name, "duplicate named argument"));
            }
        } else {
            positional.push(index);
        }
        expressions.push(expr);
    }
    let explicit_count = expressions.len();
    let mut used = vec![false; explicit_count];
    let mut next = 0usize;
    let (pieces, _span) = if let Some(format) = format {
        let span = format.span();
        let text = format.value();
        let mut chars = text.chars().peekable();
        let mut pieces = Vec::new();
        let mut chunk = String::new();
        while let Some(ch) = chars.next() {
            match ch {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    chunk.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    chunk.push('}');
                }
                '{' => {
                    if !chunk.is_empty() {
                        pieces.push(Piece::Text(mem::take(&mut chunk)));
                    }
                    let mut field = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some('{') | None => {
                                return Err(syn::Error::new(span, "unmatched opening brace"));
                            }
                            Some(ch) => field.push(ch),
                        }
                    }
                    let (argument, spec) = field.split_once(':').unwrap_or((&field, ""));
                    let index = if argument.is_empty() {
                        let index = positional.get(next).copied().ok_or_else(|| {
                            syn::Error::new(span, "not enough positional display arguments")
                        })?;
                        next += 1;
                        index
                    } else if argument.bytes().all(|ch| ch.is_ascii_digit()) {
                        argument
                            .parse::<usize>()
                            .ok()
                            .and_then(|index| positional.get(index).copied())
                            .ok_or_else(|| {
                                syn::Error::new(span, "display argument index out of range")
                            })?
                    } else {
                        let mut name = syn::parse_str::<Ident>(argument).map_err(|_| {
                            syn::Error::new(span, "expected an argument index or identifier")
                        })?;
                        name.set_span(span);
                        let key = name_key(&name);
                        if let Some(index) = names.get(&key) {
                            *index
                        } else {
                            let index = expressions.len();
                            expressions.push(syn::parse_quote!(#name));
                            names.insert(key, index);
                            index
                        }
                    };
                    if index < explicit_count {
                        used[index] = true;
                    }
                    pieces.push(Piece::Arg {
                        index,
                        spec: conversion(spec, span)?,
                    });
                }
                '}' => return Err(syn::Error::new(span, "unmatched closing brace")),
                '%' => {
                    chunk.push('%');
                    chunk.push('%');
                }
                ch => chunk.push(ch),
            }
        }
        if !chunk.is_empty() {
            pieces.push(Piece::Text(chunk));
        }
        (pieces, span)
    } else {
        if explicit_count == 0 {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected a display value",
            ));
        }
        used[0] = true;
        (
            vec![Piece::Arg {
                index: 0,
                spec: Spec {
                    hardware: "%0h".to_owned(),
                    width: 0,
                    kind: 'x',
                },
            }],
            Span::call_site(),
        )
    };
    for (index, used) in used.iter().enumerate() {
        if !used {
            return Err(syn::Error::new_spanned(
                &expressions[index],
                "unused explicit display argument",
            ));
        }
    }
    let mut statements = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) => statements.push(quote! { __display_fmt.push_str(#text); }),
            Piece::Arg { index, spec } => {
                let expr = &expressions[index];
                let Spec {
                    hardware,
                    width,
                    kind,
                } = spec;
                statements.push(quote! {
                    (#expr).push_display_arg(
                        &mut __display_fmt,
                        &mut __display_args,
                        &#crate_::__::DisplaySpec {
                            hardware: ::std::string::String::from(#hardware),
                            width: #width,
                            kind: #kind,
                        },
                    );
                });
            }
        }
    }
    Ok(quote! {{
        use #crate_::__::{DisplayArgHardware as _, DisplayArgText as _};
        let mut __display_fmt = ::std::string::String::new();
        let mut __display_args = ::std::vec::Vec::new();
        #(#statements)*
        #crate_::__::display(&__display_fmt, &__display_args)
    }})
}

pub fn display_format(input: TokenStream) -> syn::Result<TokenStream> {
    expand(syn::parse2(input)?)
}
