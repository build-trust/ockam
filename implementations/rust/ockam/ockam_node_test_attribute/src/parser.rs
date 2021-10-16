use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::PatIdent;

use crate::args;
use crate::args::{Args, TestArgs};
use crate::hygiene;

pub(crate) fn node(
    input: syn::ItemFn,
    args: syn::AttributeArgs,
) -> Result<TokenStream, syn::Error> {
    let (input, ctx_pat) = hygiene::node(input)?;
    let args = args::node(args)?;
    output_node(input, args, ctx_pat)
}

pub(crate) fn node_test(
    input: syn::ItemFn,
    args: syn::AttributeArgs,
) -> Result<TokenStream, syn::Error> {
    let (input, ctx_pat) = hygiene::node_test(input)?;
    let args = args::node_test(args)?;
    let last_stmt_end_span = {
        let mut last_stmt = input
            .block
            .stmts
            .last()
            .map(ToTokens::into_token_stream)
            .unwrap_or_default()
            .into_iter();
        // `Span` on stable Rust has a limitation that only points to the first
        // token, not the whole tokens. We can work around this limitation by
        // using the first/last span of the tokens like
        // `syn::Error::new_spanned` does.
        let start = last_stmt.next().map_or_else(Span::call_site, |t| t.span());
        last_stmt.last().map_or(start, |t| t.span())
    };
    output_node_test(input, args, last_stmt_end_span, ctx_pat)
}

fn output_node(
    input: syn::ItemFn,
    _args: Args,
    ctx_pat: PatIdent,
) -> Result<TokenStream, syn::Error> {
    let body = &input.block;
    let ctx_ident = &ctx_pat.ident;
    #[cfg(feature = "std")]
    let output = quote! {
        fn main() -> ockam::Result<()> {
            let (mut #ctx_ident, mut executor) = ockam::start_node();
            executor.execute(async move {
                #body
            })
        }
    };
    #[cfg(not(feature = "std"))]
    let output = quote! {
        fn main() -> ockam_core::Result<()> {
            let (mut #ctx_ident, mut executor) = ockam_node::start_node();
            executor.execute(async move {
                #body
            })
        }
        main().unwrap();
    };
    Ok(output.into())
}

fn output_node_test(
    mut input: syn::ItemFn,
    args: TestArgs,
    last_stmt_end_span: Span,
    ctx_pat: PatIdent,
) -> Result<TokenStream, syn::Error> {
    let body = &input.block;
    let ctx_ident = &ctx_pat.ident;
    let ctx_stop_stmt =
        quote! { #ctx_ident.stop().await.expect("Failed to stop the node context"); };
    let timeout_ms = args.timeout_ms;
    input.block = syn::parse2(quote_spanned! {last_stmt_end_span=>
        {
            use core::time::Duration;
            use tokio::time::timeout;

            let (mut #ctx_ident, mut executor) = ockam_node::start_node();
            executor
                .execute(async move {
                    match timeout(Duration::from_millis(#timeout_ms as u64), async #body).await {
                        Err(err) => {
                            #ctx_stop_stmt
                            err
                        },
                        Ok(_) => Ok(()),
                    };
                })
                .expect("Executor should not fail")
                .expect("Test function should not fail");
        }
    })
    .expect("Parsing failure");
    let header = quote! { #[::core::prelude::v1::test] };
    let output = quote! {
        #[::core::prelude::v1::test]
        #header
        #input
    };
    Ok(output.into())
}
