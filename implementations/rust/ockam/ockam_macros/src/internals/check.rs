use syn::{ItemFn, ReturnType, Type};

use crate::internals::{ast::FnVariable, ctx::Context};

pub(crate) mod item_fn {
    use super::*;

    #[cfg(not(feature = "no_main"))]
    pub(crate) fn ident_is_main(ctx: &Context, input_fn: &ItemFn) {
        if input_fn.sig.ident != "main" {
            let msg = "the function name must be `main`";
            ctx.error_spanned_by(&input_fn.sig.ident, msg);
        }
    }

    pub(crate) fn is_async(ctx: &Context, input_fn: &ItemFn) {
        if input_fn.sig.asyncness.is_none() {
            let msg = "the `async` keyword is missing from the function declaration";
            ctx.error_spanned_by(input_fn.sig.fn_token, msg);
        }
    }

    pub(crate) fn has_one_arg(ctx: &Context, input_fn: &ItemFn) {
        if input_fn.sig.inputs.len() != 1 {
            let msg = "the function must have exactly one argument";
            ctx.error_spanned_by(&input_fn.sig.inputs, msg);
        }
    }

    pub(crate) fn has_ockam_ctx_arg<'a>(
        ctx: &Context,
        input_fn: &'a ItemFn,
        ockam_ctx: &'a Option<FnVariable<'a>>,
    ) -> &'a Option<FnVariable<'a>> {
        if ockam_ctx.is_none() {
            let msg = "the function has no `Context` argument";
            ctx.error_spanned_by(&input_fn.sig.inputs, msg);
        }
        ockam_ctx
    }

    pub(crate) fn ockam_ctx_is_mut_ref<'a>(ctx: &Context, ockam_ctx: &Option<FnVariable<'a>>) {
        if let Some(ockam_ctx) = ockam_ctx {
            if ockam_ctx.and_token.is_none() {
                let msg = "the `Context` argument must be passed as reference";
                ctx.error_spanned_by(ockam_ctx.arg, msg);
            }
            if ockam_ctx.mutability.is_none() {
                let msg = "the `Context` argument must be mutable";
                ctx.error_spanned_by(ockam_ctx.arg, msg);
            }
        }
    }

    pub(crate) fn returns_result(ctx: &Context, input_fn: &ItemFn) {
        let msg = "the function must have a return type";
        match &input_fn.sig.output {
            // If the return type is the default type `()`, an error is registered.
            ReturnType::Default => {
                ctx.error_spanned_by(&input_fn.sig, msg);
            }
            ReturnType::Type(_, ret_ty) => match ret_ty.as_ref() {
                // If it's a Path we check its identifier.
                Type::Path(ty_p) => {
                    // Return error if the return type is not a type that
                    // contains `Result` in its identifier.
                    if !ty_p
                        .path
                        .segments
                        .iter()
                        .any(|s| s.ident.to_string().contains("Result"))
                    {
                        ctx.error_spanned_by(ret_ty, msg);
                    }
                }
                // In any other case, the return type is not valid.
                _ => {
                    ctx.error_spanned_by(ret_ty, msg);
                }
            },
        }
    }
}
