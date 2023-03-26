use proc_macro::TokenStream;

use quote::quote;
use syn::{Error, ItemFn};

pub(crate) fn expand(input_fn: ItemFn) -> Result<TokenStream, Error> {
    let original_fn_ident = input_fn.sig.ident;
    let import_test = quote! { use ockam_core::vault::test_support::#original_fn_ident; };
    let run_test = quote! { #original_fn_ident(&mut vault).await; };
    let output = quote! {
        #[tokio::test]
        async fn #original_fn_ident() {
            #import_test
            let mut vault = new_vault();
            #run_test
        }
    };
    Ok(output.into())
}
