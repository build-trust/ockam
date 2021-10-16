use quote::quote;
use syn::{punctuated::Punctuated, FnArg, Pat, PatIdent, Path, ReturnType, Type, TypePath};

pub(crate) fn node(input: syn::ItemFn) -> Result<(syn::ItemFn, PatIdent), syn::Error> {
    has_one_arg(&input)?;
    let (ctx_pat, _ctx_type) = arg_is_ctx(&input)?;
    ctx_is_used(&input, &ctx_pat)?;
    fn_name_is_main(&input)?;
    Ok((cleanup(input)?, ctx_pat))
}

pub(crate) fn node_test(input: syn::ItemFn) -> Result<(syn::ItemFn, PatIdent), syn::Error> {
    has_one_arg(&input)?;
    let (ctx_pat, ctx_type) = arg_is_ctx(&input)?;
    ctx_is_mut_ref(&input, &ctx_type)?;
    returns_result(&input)?;
    Ok((cleanup(input)?, ctx_pat))
}

fn has_one_arg(input: &syn::ItemFn) -> Result<(), syn::Error> {
    if input.sig.inputs.len() != 1 {
        let msg = "the function must have exactly one argument";
        return Err(syn::Error::new_spanned(&input.sig.fn_token, msg));
    }
    Ok(())
}

fn arg_is_ctx(input: &syn::ItemFn) -> Result<(PatIdent, Type), syn::Error> {
    // Capture the identifier of the argument.
    let function_arg = input.sig.inputs.first().expect("Input has no inputs");
    let (pat, ty) = match function_arg {
        FnArg::Typed(function_arg) => (function_arg.pat.as_ref(), function_arg.ty.as_ref()),
        FnArg::Receiver(_) => {
            // Passed parameter is a `self`.
            let msg = "Input argument should be of type `ockam::Context`";
            return Err(syn::Error::new_spanned(function_arg, msg));
        }
    };
    let ctx_pat = match pat {
        Pat::Ident(ident) => ident,
        _ => {
            let msg = format!(
                "Expected an identifier, found `{}`",
                quote! {#pat}.to_string()
            );
            return Err(syn::Error::new_spanned(pat, msg));
        }
    };
    // Verify that the type is `ockam::Context` (We only verify that the type is `Context`).
    // If it is some other context, there might be other compiler error, so that's fine.
    let parse_path = |path: &Path| match path.segments.last() {
        None => {
            let msg = "Input argument should be of type `ockam::Context`";
            Err(syn::Error::new_spanned(&path, msg))
        }
        Some(seg) => {
            let ident = seg.ident.to_string();
            if ident != "Context" {
                let path_ident = quote! {#path}.to_string().replace(' ', "");
                let msg = format!("Expected `ockam::Context` found `{}`", &path_ident);
                return Err(syn::Error::new_spanned(&path, msg));
            }
            Ok(())
        }
    };
    match ty {
        Type::Path(ty) => parse_path(&ty.path)?,
        Type::Reference(ty) => {
            if let Type::Path(TypePath { qself: _, path }) = &*ty.elem {
                parse_path(path)?
            } else {
                let msg = format!("Unexpected argument type {:?}", ty);
                return Err(syn::Error::new_spanned(ty, msg));
            }
        }
        _ => {
            let msg = format!("Unexpected argument type {:?}", ty);
            return Err(syn::Error::new_spanned(ty, msg));
        }
    };
    // Function body cannot be empty (Special case of unused `context`).
    if input.block.stmts.is_empty() {
        let msg = "Function body Cannot be Empty.";
        return Err(syn::Error::new_spanned(&input.sig.ident, msg));
    }
    Ok((ctx_pat.clone(), ty.clone()))
}

fn ctx_is_used(input: &syn::ItemFn, ctx_pat: &PatIdent) -> Result<(), syn::Error> {
    let mut ctx_used = false;
    for st in &input.block.stmts {
        let stmt_str = quote! {#st}.to_string().replace(' ', "");
        if stmt_str.contains(&ctx_pat.ident.to_string()) {
            ctx_used = true;
        }
    }
    if !ctx_used {
        let msg = format!(
            "Unused `{}`. Passed `ockam::Context` should be used.",
            &ctx_pat.ident.to_string()
        );
        return Err(syn::Error::new_spanned(&ctx_pat.ident, msg));
    }
    Ok(())
}

fn fn_name_is_main(input: &syn::ItemFn) -> Result<(), syn::Error> {
    if input.sig.ident != "main" {
        let msg = "The function name must be `main`";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }
    Ok(())
}

fn ctx_is_mut_ref(input: &syn::ItemFn, ctx_type: &Type) -> Result<(), syn::Error> {
    match ctx_type {
        Type::Reference(ty) => {
            if ty.mutability.is_none() {
                let msg = "The context argument must be mutable";
                return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
            }
        }
        _ => {
            let msg = "The context argument must be passed as reference";
            return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
        }
    }
    Ok(())
}

fn returns_result(input: &syn::ItemFn) -> Result<(), syn::Error> {
    if input.sig.output == ReturnType::Default {
        let msg = "The test function must have a return type";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }
    match &input.sig.output {
        ReturnType::Default => {
            let msg = "The test function must have a return type";
            return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
        }
        ReturnType::Type(_, return_type) => match return_type.as_ref() {
            Type::Path(p) => {
                let returns_result = p.path.segments.iter().any(|s| {
                    let ident = &s.ident;
                    let type_ident = quote! {#ident}.to_string();
                    type_ident == "Result"
                });
                if !returns_result {
                    let msg = "The test function must return a Result";
                    return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
                }
            }
            _ => {
                let msg = "The test function must return a Result";
                return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
            }
        },
    }
    Ok(())
}

fn cleanup(mut input: syn::ItemFn) -> Result<syn::ItemFn, syn::Error> {
    // Remove the arguments
    input.sig.inputs = Punctuated::new();
    // Remove the output
    input.sig.output = ReturnType::Default;
    // Try to remove the async keyword and fail if the function doesn't have it
    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }
    Ok(input)
}
