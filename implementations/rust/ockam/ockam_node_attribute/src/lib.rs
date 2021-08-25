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
use syn::{self, parse_macro_input, Error, Ident, ItemFn, ReturnType};

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

    // Verify that the type of the passed argument is Context
    // Capture the identifier of the argument.
    let ctx_ident: &Ident;
    let function_arg = &input_function.sig.inputs.first().unwrap();
    if let syn::FnArg::Typed(syn::PatType {
        attrs: _,
        pat,
        colon_token: _,
        ty,
    }) = function_arg
    {
        // Verify that we are passed `(ident: Type)` as a parameter.
        if let syn::Pat::Ident(syn::PatIdent {
            attrs: _,
            by_ref: _,
            mutability: _,
            ident,
            subpat: _,
        }) = &**pat
        {
            ctx_ident = ident;
        } else {
            let message = format!(
                "Expected an identifier, found `{}`",
                quote! {#pat}.to_string()
            );
            return Error::new_spanned(pat, message).to_compile_error().into();
        };

        // Verify that the type is `ockam::Context` (We only verify that the type is `Context`).
        // If it is some other context, there might be other compiler error, so that's fine.
        if let syn::Type::Path(syn::TypePath { qself: _, path }) = &**ty {
            let ident = path.segments.last();
            if ident.is_none() {
                let message = "Input argument should be of type `ockam::Context`";
                return Error::new_spanned(path, message).to_compile_error().into();
            } else {
                let type_ident = quote! {#ident}.to_string();
                if type_ident != "Context" {
                    let path_ident = quote! {#path}.to_string().replace(' ', "");
                    let message = format!("Expected `ockam::Context` found `{}`", path_ident);
                    return Error::new_spanned(path, message).to_compile_error().into();
                }
            }
        }

        // Function body cannot be empty (Special case of unused `context`).
        if input_function.block.stmts.is_empty() {
            let fn_ident = input_function.sig.ident;
            let message = "Function body Cannot be Empty.";
            return Error::new_spanned(fn_ident, message)
                .to_compile_error()
                .into();
        }

        // Make Sure that the passed Context is used.
        let mut ctx_used = false;
        for st in &input_function.block.stmts {
            let stmt_str = quote! {#st}.to_string().replace(' ', "");
            if stmt_str.contains(&ctx_ident.to_string()) {
                ctx_used = true;
            }
        }
        if !ctx_used {
            let message = format!(
                "Unused `{}`. Passed `ockam::Context` should be used.",
                &ctx_ident.to_string()
            );
            return Error::new_spanned(ctx_ident, message)
                .to_compile_error()
                .into();
        }
    } else {
        // Passed parameter is a `self`.
        let message = "Input argument should be of type `ockam::Context`";
        return Error::new_spanned(function_arg, message)
            .to_compile_error()
            .into();
    };

    // Transform the input_function to the output_function:
    // - Rename the user function
    // - Keep the same attributes, ident, inputs and output
    // - Generate a new main function with executor initialization
    // - Call the renamed user function via async/ await

    let output_fn_ident = Ident::new("trampoline", input_function.sig.ident.span());
    input_function.sig.ident = output_fn_ident.clone();
    let returns_unit = input_function.sig.output == ReturnType::Default;

    let input_function_call = if returns_unit {
        quote! {
            #output_fn_ident(#ctx_ident).await;
        }
    } else {
        quote! {
            #output_fn_ident(#ctx_ident).await.unwrap();
        }
    };

    #[cfg(feature = "std")]
    let output_function = quote! {
        #[inline(always)]
        #input_function

        fn main() -> ockam::Result<()> {
            let (#ctx_ident, mut executor) = ockam::start_node();
            executor.execute(async move {
                #input_function_call
            })
        }
    };

    #[cfg(not(feature = "std"))]
    let output_function = quote! {
        #[inline(always)]
        #input_function

        fn main() -> ockam::Result<()> {
            let (#ctx_ident, mut executor) = ockam::start_node();
            executor.execute(async move {
                #input_function_call
            })
        }
        main().unwrap();
    };

    // Create a token stream of the transformed output_function and return it.
    TokenStream::from(output_function)
}
