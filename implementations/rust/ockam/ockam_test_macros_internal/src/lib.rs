extern crate proc_macro;

use proc_macro::TokenStream;

mod vault_attribute;

#[proc_macro_attribute]
pub fn vault_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    vault_attribute::vault_test_entry(_attr, item)
}

#[proc_macro_attribute]
pub fn vault_test_sync(_attr: TokenStream, item: TokenStream) -> TokenStream {
    vault_attribute::vault_test_sync_entry(_attr, item)
}
