extern crate proc_macro;

use self::proc_macro::{Delimiter, TokenStream, TokenTree};
use core::str::FromStr;

#[proc_macro_attribute]
pub fn node(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = item.into_iter().peekable();
    let mut saw_pub = false;
    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() == "pub" {
            saw_pub = true;
        } else if t.to_string() != "async" {
            panic!("Expected \"async\"")
        }
    } else {
        panic!("Expected \"async\"")
    }

    if saw_pub {
        if let Some(TokenTree::Ident(t)) = input.next() {
            if t.to_string() != "async" {
                panic!("Expected \"async\"")
            }
        } else {
            panic!("Expected \"async\"")
        }
    }

    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() != "fn" {
            panic!("Expected \"fn\"")
        }
    } else {
        panic!("Expected \"fn\"")
    }
    let function_name = if let Some(TokenTree::Ident(t)) = input.next() {
        t.to_string()
    } else {
        panic!("Expected function name")
    };
    let func_params = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Parenthesis {
            t.stream().to_string()
        } else {
            panic!("Expected function parameters")
        }
    } else {
        panic!("Expected function parameters")
    };
    let function_content = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Brace {
            t.stream().to_string()
        } else {
            panic!("Expected function content")
        }
    } else {
        panic!("Expected function content")
    };
    let func = init_block(function_name, func_params, function_content);
    TokenStream::from_str(&*func).unwrap()
}

#[cfg(feature = "executor")]
fn init_block(function_name: String, function_params: String, function_content: String) -> String {
    format!(
        "fn {}({}) {{
        executor::block_on(async move{{ 
            {}   
       }})
    }}",
        function_name, function_params, function_content
    )
}

#[cfg(not(feature = "async"))]
fn init_block(_: String, _: String, _: String) -> String {
    panic!("An async/await environment is not present. Add the 'executor_async' feature to your ockam dependency.")
}
