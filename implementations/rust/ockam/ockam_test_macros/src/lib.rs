#![allow(clippy::unnecessary_wraps)]
#![deny(
// missing_docs,
// dead_code,
trivial_casts,
trivial_numeric_casts,
unsafe_code,
unused_import_braces,
unused_qualifications
)]

use proc_macro::TokenStream;

mod node_test_attribute;

/// Marks an async test function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node_test(args: TokenStream, item: TokenStream) -> TokenStream {
    node_test_attribute::entry(args, item)
}
