use proc_macro::TokenStream;

use quote::quote;
use syn::{Error, ItemFn};

pub(crate) fn expand(input_fn: ItemFn) -> Result<TokenStream, Error> {
    let original_fn_ident = input_fn.sig.ident;
    let import_test = quote! { use ockam_vault_test_suite::#original_fn_ident; };
    let run_test = quote! { #original_fn_ident(&mut vault).await; };
    let output = quote! {
        #[test]
        fn #original_fn_ident() {
            #import_test
            use crate::{Vault, VaultSync};

            let (mut ctx, mut executor) = ockam_node::start_node();
            executor
            .execute(async move {
                let vault = new_vault();
                let mut vault = VaultSync::create(&ctx, vault).await.unwrap();
                #run_test
                ctx.stop().await.unwrap()
            })
         .unwrap();
        }
    };
    Ok(output.into())
}
