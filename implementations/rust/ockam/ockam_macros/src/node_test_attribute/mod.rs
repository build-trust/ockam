//! The `#[ockam::test]` macro transform an async input function into a test
//! output function that sets up an ockam node and executes the body of
//! the input function inside the node.

use proc_macro::TokenStream;

mod args;
mod hygiene;
mod parser;

pub(crate) fn entry(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    for attr in &input.attrs {
        if attr.path.is_ident("test") {
            let msg = "second test attribute is supplied";
            return syn::Error::new_spanned(&attr, msg)
                .to_compile_error()
                .into();
        }
    }

    parser::node_test(input, args).unwrap_or_else(|e| e.to_compile_error().into())
}
