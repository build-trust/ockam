//! This crate provides shared macros to:
//!
//!  - faillable clone structs
//!  - create an ockam node and access its `Context`
//!  - write some node-related tests
//!
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

use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn};

mod internals;
mod message_derive;
mod node_attribute;
mod node_test_attribute;
mod try_clone_derive;
mod vault_test_attribute;

/// Implements the [`TryClone`](https://docs.rs/ockam_core/latest/ockam_core/traits/trait.TryClone.html) trait for a type.
///
/// The macro supports the following attributes:
///
/// - `#[try_clone(crate = "...")]`: specify a path to the crate that
///   will be used to import the items required by the macro. This can be
///   helpful when using the macro from an internal `ockam` crate. Defaults
///   to `ockam`.
///
/// Example of use:
///
/// ```ignore
/// #[derive(ockam::TryClone)]
/// #[try_clone(crate = "ockam")]
/// pub struct MyStruct {
///     a: u32,
/// }
/// ```
#[proc_macro_derive(TryClone, attributes(try_clone))]
pub fn try_clone_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    try_clone_derive::expand(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Implements the [`Message`](https://docs.rs/ockam_core/latest/ockam_core/trait.Message.html) trait for a type.
///
/// Example of use:
///
/// ```ignore
/// #[derive(ockam::Message, Deserialize, Serialize)]
/// pub struct MyStruct {
///     a: u32,
/// }
/// ```
#[proc_macro_derive(Message)]
pub fn message_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    message_derive::expand(input).unwrap_or_else(to_compile_error)
}

/// Marks an async function to be run in an ockam node.
///
/// The `#[node]` macro transform an async input main function into a regular
/// output main function that sets up an ockam node and executes the body of
/// the input function inside the node.
///
/// The macro supports the following attributes:
///
/// - `#[ockam::node(crate = "...")]`: specify a path to the crate that will be
///   used to import the items required by the macro. This can be helpful
///   when using the macro from an internal `ockam` crate. Defaults to
///   `ockam`.
///
/// - #[ockam::node(no_main)]: by default, this macro executes the provided
///   function within the standard entry point function `main`. If your target
///   device doesn't support this entry point, use this argument to execute the
///   input function within your own entry point as a separate function.
///
/// Example of use:
///
/// ```ignore
/// #[ockam::node]
/// async fn main(mut ctx: ockam::Context) -> ockam::Result<()> {
///     ctx.shutdown_node().await
/// }
/// ```
#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = syn::parse_macro_input!(item as ItemFn);
    node_attribute::expand(input_fn, &args.into())
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Marks an async test function to be run in an ockam node.
///
/// It transforms an async input function into a test output function that sets
/// up an ockam node and executes the body of the input function inside the
/// node.
///
/// The macro supports the following attributes:
///
/// - `#[ockam::test(crate = "...")]`: specify a path to the crate that will be
///   used to import the items required by the macro. This can be helpful
///   when using the macro from an internal `ockam` crate. Defaults to
///   `ockam_node`.
///
/// - `#[ockam::test(timeout = 1000)]`: the macro executes the test with a
///   timeout interval (in milliseconds) to avoid blocking the test
///   indefinitely. If the test times out it will panic. Defaults to 30000 (30
///   seconds).
///
/// Example of use:
///
/// ```ignore
/// #[ockam::test]
/// async fn main(ctx: &mut ockam::Context) -> ockam::Result<()> {
///     ctx.shutdown_node().await
/// }
/// ```
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = syn::parse_macro_input!(item as ItemFn);
    node_test_attribute::expand(input_fn, &args.into())
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Expands to a test suite for a custom implementation of the vault traits.
///
/// The name of the test function must match one of the test functions
/// accompanying the traits definitions in the `ockam_vault::traits` module.
///
/// Example of use:
///
/// ```ignore
/// use ockam_vault::Vault;
///
/// fn new_vault() -> Vault {
///     Vault::default()
/// }
///
/// #[ockam_macros::vault_test]
/// fn hkdf() {}
/// ```
#[proc_macro_attribute]
pub fn vault_test(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    vault_test_attribute::expand(input_fn).unwrap_or_else(to_compile_error)
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}

fn to_compile_error(error: syn::Error) -> TokenStream {
    error.to_compile_error().into()
}

#[proc_macro]
pub fn migrate(input: TokenStream) -> TokenStream {
    use syn::LitStr;

    let input = syn::parse_macro_input!(input as LitStr);

    quote! {
        {
            let migrator = sqlx::sqlx_macros::migrate!(#input);
            Migrator::new(migrator)
        }
    }
    .into()
}
