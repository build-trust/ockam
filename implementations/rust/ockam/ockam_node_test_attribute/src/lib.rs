//! Node attributes proc_macros.
//!
//! The `#[node]` macro transform an async input main function into a regular
//! output main function that sets up an ockam node and executes the body of
//! the input function inside the node.
//!
//! The `#[node_test]` macro transform an async input function into a test
//! output function that sets up an ockam node and executes the body of
//! the input function inside the node.

#![allow(clippy::unnecessary_wraps)]
#![deny(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

use proc_macro::TokenStream;

mod args;
mod entry;
mod hygiene;
mod parser;

/// Marks an async function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::main(args, item)
}

/// Marks an async test function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node_test(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::test(args, item)
}
