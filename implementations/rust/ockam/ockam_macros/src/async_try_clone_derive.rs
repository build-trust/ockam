use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, token::Comma, Attribute, Data::Struct, DeriveInput, Field,
    GenericParam, Generics, Ident, Type,
};

use crate::internals::ctx::Context;

pub(crate) fn expand(input_derive: DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Context::new();
    let cont = Container::from_ast(&ctx, &input_derive)?;
    ctx.check()?;
    Ok(output(cont))
}

fn output(cont: Container) -> TokenStream {
    let struct_ident = cont.data.struct_ident;
    let (impl_generics, ty_generics, where_clause) = cont.data.generics.split_for_impl();
    let async_trait: Attribute = parse_quote!(#[ockam_core::async_trait]);
    let fields = cont.data.struct_fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! {
            #field_name
        }
    });
    let fields_outer = fields.clone();
    let fields_async_impls = cont.data.struct_fields.iter().map(|f| {
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
                Ok((#(#fields_outer),* ,))=> {
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
    quote! {
        #async_trait
        impl #impl_generics ockam_core::AsyncTryClone for #struct_ident #ty_generics #where_clause {
            #trait_fn
        }
    }
}

struct Container<'a> {
    // Macro data.
    data: Data<'a>,
}

impl<'a> Container<'a> {
    fn from_ast(ctx: &Context, input_derive: &'a DeriveInput) -> Result<Self, Vec<syn::Error>> {
        Ok(Self {
            data: Data::from_ast(ctx, input_derive)?,
        })
    }
}

struct Data<'a> {
    struct_ident: &'a Ident,
    struct_fields: &'a Punctuated<Field, Comma>,
    generics: Generics,
}

impl<'a> Data<'a> {
    fn from_ast(_ctx: &Context, input_derive: &'a DeriveInput) -> Result<Self, Vec<syn::Error>> {
        let struct_fields = Self::struct_fields(input_derive)?;
        let generics = Self::generics(input_derive, struct_fields);
        Ok(Self {
            struct_ident: &input_derive.ident,
            struct_fields,
            generics,
        })
    }

    /// Extract struct fields from `DeriveInput`'s `data` fields.
    ///
    /// This is a prerequisite that must be met before it continues
    /// processing the macro. If this function returns an error,
    /// the macro can't continue its expansion and must return.
    ///
    /// It uses an internal `Context` instance to accumulate all
    /// possible errors and show them all to the user before the
    /// early exit.
    fn struct_fields(
        input_derive: &'a DeriveInput,
    ) -> Result<&'a Punctuated<Field, Comma>, Vec<syn::Error>> {
        let ctx = Context::new();
        let sf = match &input_derive.data {
            Struct(s) => match &s.fields {
                syn::Fields::Named(f) => Some(&f.named),
                _ => {
                    ctx.error_spanned_by(&input_derive, "the struct must have named fields only");
                    None
                }
            },
            _ => {
                ctx.error_spanned_by(&input_derive, "this macro can only be used on Structs");
                None
            }
        };
        ctx.check()?;
        Ok(sf.unwrap())
    }

    /// Extends the `DeriveInput` generics with the needed types (`Send` and `Sync`).
    fn generics(
        input_derive: &'a DeriveInput,
        struct_fields: &'a Punctuated<Field, Comma>,
    ) -> Generics {
        // Get generic type params from struct definition
        let generic_tys = input_derive
            .generics
            .type_params()
            .map(|t| &t.ident)
            .collect::<Vec<_>>();

        // Types for form name: T where T is a generic type
        let simple_generic_fields = struct_fields
            .iter()
            .filter_map(|f| {
                let outer = Self::get_outer(&f.ty)?;
                if generic_tys.iter().any(|id| id.to_string() == outer) {
                    return Some(outer);
                }
                None
            })
            .collect::<Vec<_>>();

        // Types which have a generic and are not simple
        let complex_generic_fields = struct_fields
            .iter()
            .filter_map(|f| {
                if Self::has_generic(&f.ty, &generic_tys) && Self::get_inner(&f.ty).is_some() {
                    return Some(&f.ty);
                }
                None
            })
            .collect::<Vec<_>>();

        // Clone input's derive generics to modify them.
        let mut generics = input_derive.generics.clone();

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
                        .push(parse_quote!(ockam_core::AsyncTryClone));
                }
            }
        }

        // Add where bounds
        let where_clause = generics.make_where_clause();
        for ty in complex_generic_fields {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ockam_core::AsyncTryClone));
        }

        generics
    }

    // Gets the outer of a type Outer<SomeType> or Type
    fn get_outer(ty: &Type) -> Option<String> {
        match ty {
            Type::Path(tp) if tp.qself.is_none() => {
                let segments = &tp.path.segments;
                let outer_type = if segments.len() == 1 {
                    segments.first()?.ident.to_string()
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
        if let Some(inner) = Self::get_inner(ty) {
            return Self::has_generic(inner, generics_list);
        }
        if let Type::Path(tp) = ty {
            if generics_list.contains(&&tp.path.segments[0].ident) {
                return true;
            }
        }
        false
    }
}
