use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse2, AttributeArgs, Error, ItemFn};

use super::args;
use super::args::TestArgs;
use super::hygiene::{self, NodeCtx, NodeReturn};

pub(crate) fn node_test(input: ItemFn, args: AttributeArgs) -> Result<TokenStream, Error> {
    let test_input = {
        let mut test_input = input.clone();
        let inner_ident = test_input.sig.ident;
        test_input.sig.ident = Ident::new(&format!("_{}", &inner_ident), inner_ident.span());
        test_input
    };
    let (input, ret, ctx) = hygiene::node_test(input)?;
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
        // `Error::new_spanned` does.
        let start = last_stmt.next().map_or_else(Span::call_site, |t| t.span());
        last_stmt.last().map_or(start, |t| t.span())
    };
    output_node_test(input, test_input, args, last_stmt_end_span, ret, ctx)
}

fn output_node_test(
    mut input: ItemFn,
    test_input: ItemFn,
    args: TestArgs,
    last_stmt_end_span: Span,
    _ret: NodeReturn,
    ctx: NodeCtx,
) -> Result<TokenStream, Error> {
    let ctx_ident = &ctx.pat;
    let ctx_stop_stmt = quote! { let _ = #ctx_ident.stop().await; };
    let test_input_ident = &test_input.sig.ident;
    let timeout_ms = args.timeout_ms;
    input.block = parse2(quote_spanned! {last_stmt_end_span=>
        {
            use core::time::Duration;
            use ockam_node::tokio::time::timeout;

            let (mut #ctx_ident, mut executor) = ockam_node::start_node();
            executor
                .execute(async move {
                    match timeout(Duration::from_millis(#timeout_ms as u64), #test_input_ident(&mut #ctx_ident)).await {
                        Ok(r) => match r {
                            Err(err) => {
                                #ctx_stop_stmt
                                Err(err)
                            },
                            Ok(_) => Ok(())
                        },
                        Err(_) => {
                            #ctx_stop_stmt
                            panic!("Test timeout")
                        }
                    }
                })
                .expect("Executor should not fail")
                .expect("Test function should not fail");
        }
    })
    .expect("Parsing failure");
    let output = quote! {
        #test_input
        #[::core::prelude::v1::test]
        #input
    };
    Ok(output.into())
}
