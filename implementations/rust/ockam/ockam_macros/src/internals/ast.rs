use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::{And, Comma, Mut};
use syn::{FnArg, ItemFn, Pat, PatIdent, Type, TypePath};

use crate::internals::ctx::Context;

#[derive(Debug)]
pub struct FnVariable<'a> {
    pub arg: &'a FnArg,
    pub ident: &'a PatIdent,
    pub path: &'a TypePath,
    pub and_token: Option<And>,
    pub mutability: Option<Mut>,
}

impl<'a> FnVariable<'a> {
    pub fn from_fn_args(
        ctx: &Context,
        args: &'a Punctuated<FnArg, Comma>,
    ) -> Punctuated<FnVariable<'a>, Comma> {
        let mut vars: Punctuated<FnVariable, Comma> = Punctuated::new();
        for arg in args {
            match arg {
                // If the argument is a `self` variation, skip it.
                FnArg::Receiver(_) => continue,
                FnArg::Typed(ty) => {
                    // The function argument must have an identifier, otherwise, move on to the next argument.
                    let ident = match ty.pat.as_ref() {
                        Pat::Ident(ident) => ident,
                        _ => continue,
                    };
                    // Extract the argument path, mutability and reference.
                    let (path, mutability, and_token) = match ty.ty.as_ref() {
                        // e.g. `&mut ockam::Context`
                        Type::Reference(ty_ref) => {
                            if let Type::Path(ty_path) = &*ty_ref.elem {
                                (ty_path, ty_ref.mutability, Some(ty_ref.and_token))
                            } else {
                                let msg =
                                    format!("unexpected function argument type {}", quote! {#ty});
                                ctx.error_spanned_by(ty, msg);
                                continue;
                            }
                        }
                        // e.g. `ockam::Context`
                        Type::Path(ty_path) => (ty_path, ident.mutability, None),
                        // In any other case, the argument is invalid and an error is registered.
                        _ => {
                            let msg = format!("unexpected function argument type {}", quote! {#ty});
                            ctx.error_spanned_by(ty, msg);
                            continue;
                        }
                    };
                    vars.push(FnVariable {
                        arg,
                        ident,
                        path,
                        and_token,
                        mutability,
                    });
                }
            }
        }
        vars
    }
}

pub fn ockam_context_variable_from_input_fn<'a>(
    ctx: &Context,
    input_fn: &'a ItemFn,
) -> Option<FnVariable<'a>> {
    // Verify that the type of one of the function arguments is `ockam::Context`.
    // We only verify that the type path contains a segment called `Context`.
    // If it is some other context, there might be other compiler error, so that's fine.
    FnVariable::from_fn_args(ctx, &input_fn.sig.inputs)
        .into_iter()
        .find(|var| match var.path.path.segments.last() {
            None => false,
            Some(seg) => seg.ident.to_string().eq("Context"),
        })
}
