use std::fmt::Display;
use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse, parse::Parse, Lit, Path};

use crate::internals::{ctx::Context, respan::respan, symbol::Symbol};

/// A macro attribute.
///
/// From `ockam::test(timeout = 1000)`, an `Attr` instance
/// will contain:
/// - The name: `timeout`.
/// - The value: `1000`.
pub(crate) struct Attr<'c, T> {
    ctx: &'c Context,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

#[allow(dead_code)]
impl<'c, T> Attr<'c, T> {
    pub(crate) fn none(ctx: &'c Context, name: Symbol) -> Self {
        Attr {
            ctx,
            name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    pub(crate) fn set<A: ToTokens>(&mut self, obj: A, value: T) {
        let tokens = obj.into_token_stream();

        if self.value.is_some() {
            self.ctx
                .error_spanned_by(tokens, format!("duplicate attribute `{}`", self.name));
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    pub(crate) fn set_opt<A: ToTokens>(&mut self, obj: A, value: Option<T>) {
        if let Some(value) = value {
            self.set(obj, value);
        }
    }

    pub(crate) fn set_if_none(&mut self, value: T) {
        if self.value.is_none() {
            self.value = Some(value);
        }
    }

    pub(crate) fn get(self) -> Option<T> {
        self.value
    }

    pub(crate) fn get_with_tokens(self) -> Option<(TokenStream, T)> {
        match self.value {
            Some(v) => Some((self.tokens, v)),
            None => None,
        }
    }
}

pub(crate) struct BoolAttr<'c>(Attr<'c, ()>);

impl<'c> BoolAttr<'c> {
    pub(crate) fn none(cx: &'c Context, name: Symbol) -> Self {
        BoolAttr(Attr::none(cx, name))
    }

    pub(crate) fn set_true<A: ToTokens>(&mut self, obj: A) {
        self.0.set(obj, ());
    }

    pub(crate) fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

fn get_lit_str<'a>(
    ctx: &Context,
    attr_name: Symbol,
    lit: &'a syn::Expr,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: Lit::Str(lit), ..
    }) = lit
    {
        Ok(lit)
    } else {
        ctx.error_spanned_by(
            lit,
            format!(
                "expected {} attribute to be a string: `{} = \"...\"`",
                attr_name, attr_name
            ),
        );
        Err(())
    }
}

fn get_lit_int<'a>(
    ctx: &Context,
    attr_name: Symbol,
    lit: &'a syn::Expr,
) -> Result<&'a syn::LitInt, ()> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: Lit::Int(lit), ..
    }) = lit
    {
        Ok(lit)
    } else {
        ctx.error_spanned_by(
            lit,
            format!(
                "expected {} attribute to be an int: `{} = \"...\"`",
                attr_name, attr_name
            ),
        );
        Err(())
    }
}

fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan(stream, s.span()))
}

pub(crate) fn parse_lit_into_path(
    ctx: &Context,
    attr_name: Symbol,
    lit: &syn::Expr,
) -> Result<Path, ()> {
    let string = get_lit_str(ctx, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        ctx.error_spanned_by(lit, format!("failed to parse path: {:?}", string.value()));
    })
}

pub(crate) fn parse_lit_into_int<T>(
    ctx: &Context,
    attr_name: Symbol,
    lit: &syn::Expr,
) -> Result<T, ()>
where
    T: FromStr,
    T::Err: Display,
{
    let int = get_lit_int(ctx, attr_name, lit)?;
    match int.base10_parse::<T>() {
        Err(_) => {
            ctx.error_spanned_by(lit, format!("failed to parse int: {:?}", int.to_string()));
            Err(())
        }
        Ok(int) => Ok(int),
    }
}
