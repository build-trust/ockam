//! Message attribute proc_macro.
//!
//! The `#[derive(Message)]` macro implements `Message` trait for the type.
//!
//! The main Ockam crate re-exports this macro.

use proc_macro::TokenStream;
use quote::quote;

pub fn entry(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).expect("failed to parse macro input");

    // Build the trait implementation
    impl_message_macro(&ast)
}

fn impl_message_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let gen = quote! {
        impl #impl_generics Message for #name #ty_generics #where_clause {}
    };
    gen.into()
}
