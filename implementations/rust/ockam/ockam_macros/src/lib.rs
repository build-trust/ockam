#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

use proc_macro::TokenStream;

mod async_try_clone_derive;
mod message_derive;
mod node_attribute;

#[proc_macro_derive(AsyncTryClone)]
pub fn async_try_clone_derive(input: TokenStream) -> TokenStream {
    async_try_clone_derive::entry(input)
}

/// Implements Message trait for a type.
#[proc_macro_derive(Message)]
pub fn message_derive(input: TokenStream) -> TokenStream {
    message_derive::entry(input)
}

/// Marks an async function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    node_attribute::entry(args, item)
}
