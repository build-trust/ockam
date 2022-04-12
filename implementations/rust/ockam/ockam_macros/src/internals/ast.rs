use proc_macro2::Ident;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::{And, Comma, Mut};
use syn::{FnArg, ItemFn, Pat, Type, TypePath};

use crate::internals::ctx::Context;

/// A representation of a function argument that we want to use
/// as a variable when expanding a macro.
///
/// Example:
///
/// If a macro receives an `InputFn` like the following:
/// ```ignore
/// fn foo(arg: &mut std::str::Bytes) {}
/// ```
///
/// An `FnVariable` will contain the following data:
/// - The original `FnArg` that was used to extract the data from.
/// - The identifier: `arg`.
/// - The type path: `std::str::Bytes`.
/// - The reference token: `&`.
/// - The mutability token: `mut`.
pub(crate) struct FnVariable<'a> {
    pub(crate) arg: &'a FnArg,
    pub(crate) ident: &'a Ident,
    pub(crate) path: &'a TypePath,
    pub(crate) and_token: Option<And>,
    pub(crate) mutability: Option<Mut>,
}

impl<'a> FnVariable<'a> {
    /// Extracts a list of `FnVariable` items out from a list of `FnArg`,
    /// generally provided by the `InputFn`'s `sig.inputs` attribute.
    pub(crate) fn from_fn_args(
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
                        ident: &ident.ident,
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

/// This is a specific use case where we want to extract only
/// the `ockam::Context` variable from all the arguments available
/// in the `InputFn`.
pub(crate) fn ockam_context_variable_from_input_fn<'a>(
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
