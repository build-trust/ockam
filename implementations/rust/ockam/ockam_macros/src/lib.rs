#![deny(
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

//! Procedural macros for use with Ockam.

use proc_macro::TokenStream;

mod async_try_clone_derive;
mod message_derive;
mod node_attribute;
mod node_test_attribute;
mod vault_test_attribute;
mod vault_test_sync_attribute;

/// Custom derive for the `ockam_core::AsyncTryClone` trait.
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
///
/// Use of this macro requires `ockam_node`.
#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    node_attribute::entry(args, item)
}

/// Marks an async test function to be run in an ockam node.
///
/// The macro supports the following attributes:
///
/// - #[ockam::test(crate = "..."]: specify a path to the crate that will be used to import the functions required
/// by the macro. This can be helpful when using the macro from an internal `ockam` crate. Defaults to `ockam_node`.
///
/// - #[ockam::test(timeout = 1000]: the macro executes the test with a timeout interval (in milliseconds) to avoid
/// blocking the test indefinitely. If the test times out it will panic. Defaults to 30000 (30 seconds).
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    node_test_attribute::entry(args, item)
}

/// Expands to a test suite for a custom implementation of the vault traits.
#[proc_macro_attribute]
pub fn vault_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    vault_test_attribute::entry(_attr, item)
}

/// Expands to a test suite for a custom implementation of the vault traits.
#[proc_macro_attribute]
pub fn vault_test_sync(_attr: TokenStream, item: TokenStream) -> TokenStream {
    vault_test_sync_attribute::entry(_attr, item)
}
