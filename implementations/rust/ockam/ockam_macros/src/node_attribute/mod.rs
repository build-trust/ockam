//! The `#[node]` macro transform an async input main function into a regular
//! output main function that sets up an ockam node and executes the body of
//! the input function inside the node.

use proc_macro::TokenStream;

mod args;
mod hygiene;
mod parser;

pub(crate) fn entry(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    parser::node(input, args).unwrap_or_else(|e| e.to_compile_error().into())
}
