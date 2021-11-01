//! Message attribute proc_macro.
//!
//! The `#[derive(Message)]` macro implements `Message` trait for the type.
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

/// Implements Message trait for a type.
#[proc_macro_derive(Message)]
pub fn message_derive(input: TokenStream) -> TokenStream {
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
