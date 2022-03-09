//! Message attribute proc_macro.
//!
//! The `#[derive(Message)]` macro implements `Message` trait for the type.
//!
//! The main Ockam crate re-exports this macro.

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn entry(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    parse(&input)
}

fn parse(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let gen = quote! {
        impl #impl_generics Message for #name #ty_generics #where_clause {}
    };
    gen.into()
}
