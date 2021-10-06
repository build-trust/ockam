use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned, ToTokens};

use crate::args;
use crate::args::Args;
use crate::hygiene;

pub(crate) fn parse_macro(
    input: syn::ItemFn,
    args: syn::AttributeArgs,
    is_test: bool,
) -> Result<TokenStream, syn::Error> {
    let (input, ctx_pat) = hygiene::input(input, is_test)?;
    let args = args::parse(args, is_test)?;
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
    let ctx_ident = &ctx_pat.ident;
    if is_test {
        output_node_test(input, args, last_stmt_end_span, ctx_ident)
    } else {
        output_node(input, ctx_ident)
    }
}

fn output_node_test(
    mut input: syn::ItemFn,
    args: Args,
    last_stmt_end_span: Span,
    ctx_ident: &Ident,
) -> Result<TokenStream, syn::Error> {
    let body = &input.block;
    match args.timeout_ms {
        Some(timeout) => {
            input.block = syn::parse2(quote_spanned! {last_stmt_end_span=>
                {
                    use core::time::Duration;
                    use tokio::time::timeout;

                    let (tx, rx) = std::sync::mpsc::channel::<bool>();
                    let (mut #ctx_ident, mut executor) = ockam_node::start_node();
                    executor
                        .execute(async move {
                            let is_ok = timeout(Duration::from_millis(#timeout as u64), async #body).await.is_ok();
                            tx.send(is_ok).expect("Failed to send test result");
                            if !is_ok {
                                #ctx_ident.stop().await.expect("Failed to stop the node context");
                            }
                        })
                        .expect("Executor failed");
                    let test_res = rx.try_recv().expect("Failed to receive test response from executor");
                    assert!(test_res, "Test timeout reached");
                }
            })
                .expect("Parsing failure");
        }
        _ => {
            input.block = syn::parse2(quote_spanned! {last_stmt_end_span=>
                {
                    let (mut #ctx_ident, mut executor) = ockam_node::start_node();
                    executor
                    .execute(async move {
                        let res = #body;
                        if res.is_err() {
                            #ctx_ident.stop().await.expect("Failed to stop the node context");
                        }
                    })
                    .expect("Executor failed");
                }
            })
            .expect("Parsing failure");
        }
    }
    let header = quote! { #[::core::prelude::v1::test] };
    let output = quote! {
        #header
        #input
    };
    Ok(output.into())
}

fn output_node(input: syn::ItemFn, ctx_ident: &Ident) -> Result<TokenStream, syn::Error> {
    let body = &input.block;
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
