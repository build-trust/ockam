extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, ItemFn, Stmt};

#[proc_macro_attribute]
pub fn vault_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_fn = parse_macro_input!(item as ItemFn);
    let original_fn_ident = original_fn.clone().sig.ident;
    let import_test = TokenStream::from_str(
        format!(
            "use ockam_vault_test_suite::{};",
            original_fn_ident.to_string()
        )
        .as_str(),
    )
    .unwrap();
    let import_test: Stmt = syn::parse(import_test.into()).expect("B");
    let run_test =
        TokenStream::from_str(format!("{}(&mut vault);", original_fn_ident.to_string()).as_str())
            .unwrap();
    let run_test: Stmt = syn::parse(run_test.into()).expect("B");

    let output_function = quote! {
        #[test]
        fn #original_fn_ident() {
            #import_test
            let mut vault = new_vault();
            #run_test
        }
    };

    TokenStream::from(output_function)
}

#[proc_macro_attribute]
pub fn vault_test_sync(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_fn = parse_macro_input!(item as ItemFn);
    let original_fn_ident = original_fn.clone().sig.ident;
    let import_test = TokenStream::from_str(
        format!(
            "use ockam_vault_test_suite::{};",
            original_fn_ident.to_string()
        )
        .as_str(),
    )
    .unwrap();
    let import_test: Stmt = syn::parse(import_test.into()).expect("B");
    let run_test =
        TokenStream::from_str(format!("{}(&mut vault);", original_fn_ident.to_string()).as_str())
            .unwrap();
    let run_test: Stmt = syn::parse(run_test.into()).expect("B");

    let output_function = quote! {
        #[test]
        fn #original_fn_ident() {
            #import_test
            use crate::{Vault, VaultWorker};

            let (mut ctx, mut executor) = ockam_node::start_node();
            executor
            .execute(async move {
                let vault = new_vault();
                let vault_address = VaultWorker::start(&ctx, vault)
                    .await
                    .unwrap();
                let mut vault = Vault::start(&ctx, vault_address).await.unwrap();
                #run_test

                ctx.stop().await.unwrap()
            })
         .unwrap();
        }
    };

    TokenStream::from(output_function)
}
