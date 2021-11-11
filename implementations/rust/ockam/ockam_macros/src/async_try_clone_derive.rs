use proc_macro::TokenStream;

use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data::Struct, DeriveInput, GenericParam, Ident, Type,
};

// Gets the outer of a type Outer<SomeType> or Type
fn get_outer(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => {
            let segments = &tp.path.segments;
            let outer_type = if segments.len() == 1 {
                segments.first().unwrap().ident.to_string()
            } else {
                segments.iter().fold(String::new(), |mut acc, s| {
                    acc.push_str("::");
                    acc.push_str(&s.ident.to_string());
                    acc
                })
            };
            Some(outer_type)
        }
        _ => None,
    }
}

// Gets the inner of a type SomeType<Inner> or none if it doesn't exist
fn get_inner(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => {
            let mut tp = tp
                .path
                .segments
                .iter()
                .skip_while(|s| s.arguments.is_empty());
            if let Some(segment) = tp.next() {
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(ab) if ab.args.len() == 1 => {
                        if let Some(syn::GenericArgument::Type(t)) = ab.args.first() {
                            return Some(t);
                        }
                        return None;
                    }
                    _ => return None,
                }
            }
            None
        }
        _ => None,
    }
}

fn has_generic(ty: &Type, generics_list: &[&Ident]) -> bool {
    if let Some(inner) = get_inner(ty) {
        return has_generic(inner, generics_list);
    }
    if let Type::Path(tp) = ty {
        if generics_list.contains(&&tp.path.segments[0].ident) {
            return true;
        }
    }
    false
}

pub fn entry(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_data = match ast.data {
        Struct(s) => s,
        _ => {
            panic!("currently only works for structs");
        }
    };
    let struct_ident = ast.ident;
    let named_fields = match struct_data.fields {
        syn::Fields::Named(f) => f.named,
        _ => {
            panic!("currently only works for named fields");
        }
    };

    // Get generic type params from struct definition
    let generic_tys = ast
        .generics
        .type_params()
        .map(|t| &t.ident)
        .collect::<Vec<_>>();

    // Types for form name: T where T is a generic type
    let simple_generic_fields = named_fields
        .iter()
        .filter_map(|f| {
            let outer = get_outer(&f.ty).unwrap();
            if generic_tys.iter().any(|id| id.to_string() == outer) {
                return Some(outer);
            }
            None
        })
        .collect::<Vec<_>>();

    // Types which have a generic and are not simple
    let complex_generic_fields = named_fields
        .iter()
        .filter_map(|f| {
            if has_generic(&f.ty, &generic_tys) && get_inner(&f.ty).is_some() {
                return Some(&f.ty);
            }
            None
        })
        .collect::<Vec<_>>();

    let mut generics = ast.generics;

    // Add trait bounds on generic type params
    for p in &mut generics.params {
        if let GenericParam::Type(ref mut t) = *p {
            // Every generic type must be Send + Sync
            t.bounds.push(parse_quote!(::core::marker::Send));
            t.bounds.push(parse_quote!(::core::marker::Sync));

            // Generic simple type must also be AsyncTryClone
            if simple_generic_fields
                .iter()
                .any(|s| s == &t.ident.to_string())
            {
                t.bounds
                    .push(parse_quote!(ockam_core::traits::AsyncTryClone));
            }
        }
    }

    // Add where bounds
    let where_clause = generics.make_where_clause();
    for ty in complex_generic_fields {
        where_clause
            .predicates
            .push(parse_quote!(#ty: ockam_core::traits::AsyncTryClone));
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let async_trait: Attribute = parse_quote!(#[ockam_core::async_trait]);
    let fields = named_fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! {
            #field_name
        }
    });
    let fields_clone = fields.clone();
    let fields_async_impls = named_fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! {
            self.#field_name.async_try_clone()
        }
    });
    let trait_fn = quote! {
        async fn async_try_clone(&self) -> ockam_core::Result<Self>{
            let results = ockam_core::compat::try_join!(
                #(#fields_async_impls),*
            );
            match results {
                Ok((#(#fields_clone),* ,))=> {
                    Ok(
                        Self{
                            #(#fields),*
                        }
                    )
                }
                Err(e) => {
                    Err(e)
                }
            }
        }
    };
    let output = quote! {
        #async_trait
        impl #impl_generics ockam_core::traits::AsyncTryClone for #struct_ident #ty_generics #where_clause {
            #trait_fn
        }
    };
    TokenStream::from(output)
}
