use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, Error, ItemFn};

pub(crate) fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    parse(input).unwrap_or_else(|e| e.to_compile_error().into())
}

fn parse(input: ItemFn) -> Result<TokenStream, Error> {
    let original_fn_ident = input.sig.ident;
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
