use proc_macro::TokenStream;

use quote::quote;
use syn::{AttributeArgs, Error, ItemFn, ReturnType};

use super::args;
use super::args::Args;
use super::hygiene::{self, NodeCtx, NodeReturn};

pub(crate) fn node(input: ItemFn, args: AttributeArgs) -> Result<TokenStream, Error> {
    let (input, ret, ctx) = hygiene::node(input)?;
    let args = args::node(args)?;
    output_node(input, args, ret, ctx)
}

fn output_node(
    input: ItemFn,
    args: Args,
    ret: NodeReturn,
    ctx: NodeCtx,
) -> Result<TokenStream, Error> {
    let body = &input.block;
    let ret_type = ret.ty;
    let ctx_ident = &ctx.pat.ident;
    let ctx_path = &ctx.path;
    let ctx_mut = &ctx.mutability;

    // Handles error if inner function returns Result, unwraps it otherwise.
    let err_handling = if ret_type == ReturnType::Default {
        quote! {.unwrap();}
    } else {
        #[cfg(feature = "std")]
        quote! {?}

        // For now the executor's `Executor::execute` for std returns `Result<F::Output>`
        // while for no_std it returns `Result<()>` and always returns `Ok(())` or panics.
        // So while it makes sense using the `?` operator in std in no_std just returning Ok(())
        // would be enough(since execute already hides the return type) but we are letting the
        // `Executor::execute` return bubble up to keep the code simpler.
        // Note: This also means that for no_std `main` return type can only be `Result<()>` or nothing.
        #[cfg(not(feature = "std"))]
        quote! {}
    };

    let output = if !args.no_main {
        // Assumes the target platform knows about main() functions
        quote! {
            fn main() #ret_type {
                let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (#ctx_path, ockam::Executor);
                executor.execute(async move #body)#err_handling
            }
        }
    } else {
        // Assumes you will be defining the ockam node inside your own entry point
        quote! {
            fn ockam_async_main() #ret_type {
                let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (#ctx_path, ockam::Executor);
                executor.execute(async move #body)#err_handling
            }
            // TODO: safe way to print the error before panicking?
            ockam_async_main().unwrap();
        }
    };
    Ok(output.into())
}
