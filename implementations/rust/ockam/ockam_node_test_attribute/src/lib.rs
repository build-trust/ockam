use proc_macro::TokenStream;

mod args;
mod entry;
mod hygiene;
mod parser;

#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::main(args, item)
}

#[proc_macro_attribute]
pub fn node_test(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::test(args, item)
}
