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
    _args: Args,
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
        quote! {?}
    };

    // Assumes the target platform knows about main() functions
    #[cfg(not(feature = "no_main"))]
    let output = quote! {
        fn main() #ret_type {
            let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (#ctx_path, ockam::Executor);
            executor.execute(async move #body)#err_handling
        }
    };
    // Assumes you will be defining the ockam node inside your own entry point
    #[cfg(feature = "no_main")]
    let output = quote! {
        fn ockam_async_main() #ret_type {
            let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (#ctx_path, ockam::Executor);
            executor.execute(async move #body)#err_handling
        }
        // TODO: safe way to print the error before panicking?
        ockam_async_main().unwrap();
    };
    Ok(output.into())
}
