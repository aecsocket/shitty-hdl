use std::collections::HashMap;

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

fn conversion(spec: &str, span: Span) -> syn::Result<String> {
    let width_end = spec.bytes().take_while(u8::is_ascii_digit).count();
    let (width, kind) = spec.split_at(width_end);
    if !width.is_empty() && width.parse::<u32>().is_err() {
        return Err(syn::Error::new(span, "format width is too large"));
    }
    let kind = match kind {
        "" => "d",
        "x" | "?" => "h",
        "b" => "b",
        "o" => "o",
        _ => {
            return Err(syn::Error::new(
                span,
                "unsupported display format; use an optional literal width followed by x, b, o, ?, or no type",
            ));
        }
    };
    Ok(format!(
        "%{}{kind}",
        if width.is_empty() { "0" } else { width }
    ))
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
    let mut ordered = Vec::new();
    let (bsv, span) = if let Some(format) = format {
        let span = format.span();
        let text = format.value();
        let mut chars = text.chars().peekable();
        let mut bsv = String::new();
        let mut next = 0usize;
        while let Some(ch) = chars.next() {
            match ch {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    bsv.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    bsv.push('}');
                }
                '{' => {
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
                    ordered.push(index);
                    bsv.push_str(&conversion(spec, span)?);
                }
                '}' => return Err(syn::Error::new(span, "unmatched closing brace")),
                '%' => bsv.push_str("%%"),
                ch => bsv.push(ch),
            }
        }
        for (index, used) in used.iter().enumerate() {
            if !used {
                return Err(syn::Error::new_spanned(
                    &expressions[index],
                    "unused explicit display argument",
                ));
            }
        }
        (bsv, span)
    } else {
        ordered.push(0);
        ("%0h".to_owned(), Span::call_site())
    };
    let locals: Vec<_> = (0..expressions.len())
        .map(|index| Ident::new(&format!("__display_arg_{index}"), Span::mixed_site()))
        .collect();
    let handles = ordered.iter().map(|index| &locals[*index]);
    let bsv = LitStr::new(&bsv, span);
    Ok(quote! {{
        #(let #locals = #crate_::__::display_arg(#expressions);)*
        #crate_::__::display(#bsv, &[#(#handles),*])
    }})
}

pub fn display_format(input: TokenStream) -> syn::Result<TokenStream> {
    expand(syn::parse2(input)?)
}
