use quote::quote;
use syn::{punctuated::Punctuated, FnArg, Pat, PatIdent, ReturnType};

pub(crate) fn input(
    input: syn::ItemFn,
    is_test: bool,
) -> Result<(syn::ItemFn, PatIdent), syn::Error> {
    input_has_one_arg(&input)?;
    let ctx_pat = input_arg_is_ctx(&input, is_test)?;
    input_has_return_type(&input, is_test)?;
    let input = input_cleanup(input)?;
    Ok((input, ctx_pat))
}

fn input_has_one_arg(input: &syn::ItemFn) -> Result<(), syn::Error> {
    if input.sig.inputs.len() != 1 {
        let msg = "the function must have exactly one argument";
        return Err(syn::Error::new_spanned(&input.sig.fn_token, msg));
    }
    Ok(())
}

fn input_arg_is_ctx(input: &syn::ItemFn, is_test: bool) -> Result<PatIdent, syn::Error> {
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
    if let syn::Type::Path(syn::TypePath { qself: _, path }) = &*ty {
        let ident = path.segments.last();
        if ident.is_none() {
            let msg = "Input argument should be of type `ockam::Context`";
            return Err(syn::Error::new_spanned(path, msg));
        } else {
            let type_ident = quote! {#ident}.to_string();
            if type_ident != "Context" {
                let path_ident = quote! {#path}.to_string().replace(' ', "");
                let msg = format!("Expected `ockam::Context` found `{}`", path_ident);
                return Err(syn::Error::new_spanned(path, msg));
            }
        }
    }
    // Function body cannot be empty (Special case of unused `context`).
    if input.block.stmts.is_empty() {
        let msg = "Function body Cannot be Empty.";
        return Err(syn::Error::new_spanned(&input.sig.ident, msg));
    }
    if !is_test {
        // Make Sure that the passed Context is used.
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
    }
    Ok(ctx_pat.clone())
}

fn input_has_return_type(input: &syn::ItemFn, is_test: bool) -> Result<(), syn::Error> {
    if !is_test {
        return Ok(());
    }
    if input.sig.output != ReturnType::Default {
        let msg = "the test function can't have a return type";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }
    Ok(())
}

fn input_cleanup(mut input: syn::ItemFn) -> Result<syn::ItemFn, syn::Error> {
    input.sig.inputs = Punctuated::new();
    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }
    Ok(input)
}
