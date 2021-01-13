extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error, ItemFn};

/// Marks an async function to be run in an ockam node.
#[proc_macro_attribute]
pub fn node(_args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the item that #[ockam::node] is defined on.
    // Expect that this item is a function and fail if it isn't a function
    let input_function = parse_macro_input!(item as ItemFn);

    // Fail if the function is not declared async
    if input_function.sig.asyncness.is_none() {
        let message = "a function tagged with '#[ockam::node]' must be declared as 'async'";
        let token = input_function.sig.fn_token;
        return Error::new_spanned(token, message).to_compile_error().into();
    }

    let input_function_attrs = &input_function.attrs;
    let input_function_ident = &input_function.sig.ident;
    let input_function_inputs = &input_function.sig.inputs;
    let input_function_output = &input_function.sig.output;
    let input_function_block = &input_function.block;

    // Transform the input_function to the output_function:
    // - Remove async
    // - Keep the same attributes, ident, inputs and output
    // - Put the body block of the input_functio inside an async block
    // - Invoke ockam::node::block_on() with this async block as an argument

    let output_function = quote! {
        #(#input_function_attrs)*
        fn #input_function_ident(#input_function_inputs) #input_function_output {
            ockam::node::block_on(async move {
                #input_function_block
            })
        }
    };

    // Create a token stream of the transformed output_function and return it.
    TokenStream::from(output_function)
}
