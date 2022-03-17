//! Message attribute proc_macro.
//!
//! The `#[derive(Message)]` macro implements `Message` trait for the type.
//!
//! The main Ockam crate re-exports this macro.

use proc_macro::TokenStream;

use quote::quote;
use syn::{DeriveInput, Error};

pub(crate) fn expand(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let output = quote! {
        impl #impl_generics Message for #name #ty_generics #where_clause {}
    };
    Ok(output.into())
}
