//! Node attribute proc_macro.
//!
//! The `#[node]` macro transform an async input main function into a regular
//! output main function that sets up an ockam node and executes the body of
//! the input function inside the node.
//!
//! The main Ockam crate re-exports this macro.

#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error, Ident, ItemFn};

/// Marks an async function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node(_args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the item that #[ockam::node] is defined on.
    // Expect that this item is a function and fail if it isn't a function
    let mut input_function = parse_macro_input!(item as ItemFn);

    // Fail if the function is not declared async
    if input_function.sig.asyncness.is_none() {
        let message = "a function with attribute '#[ockam::node]' must be declared as 'async'";
        let token = input_function.sig.fn_token;
        return Error::new_spanned(token, message).to_compile_error().into();
    }

    // Fail if the function does not have exactly one argument
    if input_function.sig.inputs.len() != 1 {
        let message = "a function with '#[ockam::node]' must have exactly one argument";
        let token = input_function.sig.fn_token;
        return Error::new_spanned(token, message).to_compile_error().into();
    }

    // Transform the input_function to the output_function:
    // - Rename the user function
    // - Keep the same attributes, ident, inputs and output
    // - Generate a new main function with executor initialization
    // - Call the renamed user function via async/ await

    let output_fn_ident = Ident::new("trampoline", input_function.sig.ident.span());
    input_function.sig.ident = output_fn_ident.clone();

    let output_function = quote! {
        #[inline(always)]
        #input_function

        fn main() -> ockam::Result<()> {
            let (context, mut executor) = ockam::start_node();
            executor.execute(async move { #output_fn_ident(context).await })
        }
    };
    // Create a token stream of the transformed output_function and return it.
    TokenStream::from(output_function)
}
