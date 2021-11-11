use quote::quote;
use syn::{
    punctuated::Punctuated, Error, FnArg, ItemFn, Pat, PatIdent, Path, ReturnType, Token, Type,
    TypePath,
};

pub(crate) fn node_test(input: ItemFn) -> Result<(ItemFn, NodeReturn, NodeCtx), Error> {
    let ret = NodeReturn::new(&input)?;
    ret.node_test_checks(&input)?;

    let ctx = NodeCtx::new(&input)?;
    ctx.node_test_checks(&input)?;

    Ok((cleanup(input)?, ret, ctx))
}

pub(crate) struct NodeReturn {
    pub(crate) ty: ReturnType,
}

impl NodeReturn {
    fn new(input: &ItemFn) -> Result<Self, Error> {
        Ok(Self {
            ty: input.sig.output.clone(),
        })
    }

    fn node_test_checks(&self, input: &ItemFn) -> Result<(), Error> {
        self.fn_is_async(input)?;
        self.has_one_arg(input)?;
        self.returns_result(input)?;
        Ok(())
    }

    fn fn_is_async(&self, input: &ItemFn) -> Result<(), Error> {
        if input.sig.asyncness.is_none() {
            let msg = "the `async` keyword is missing from the function declaration";
            return Err(Error::new_spanned(input.sig.fn_token, msg));
        }
        Ok(())
    }

    fn has_one_arg(&self, input: &ItemFn) -> Result<(), Error> {
        if input.sig.inputs.len() != 1 {
            let msg = "the function must have exactly one argument";
            return Err(Error::new_spanned(&input.sig.fn_token, msg));
        }
        Ok(())
    }

    fn returns_result(&self, input: &ItemFn) -> Result<(), Error> {
        let msg = "The function must have a return type";
        if self.ty == ReturnType::Default {
            return Err(Error::new_spanned(input.sig.fn_token, msg));
        }
        match &self.ty {
            ReturnType::Default => {
                return Err(Error::new_spanned(input.sig.fn_token, msg));
            }
            ReturnType::Type(_, return_type) => match return_type.as_ref() {
                Type::Path(p) => {
                    let returns_result = p.path.segments.iter().any(|s| {
                        let ident = &s.ident;
                        let type_ident = quote! {#ident}.to_string();
                        type_ident == "Result"
                    });
                    if !returns_result {
                        return Err(Error::new_spanned(input.sig.fn_token, msg));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(input.sig.fn_token, msg));
                }
            },
        }
        Ok(())
    }
}

pub(crate) struct NodeCtx {
    pub(crate) pat: PatIdent,
    ty: Type,
    pub(crate) mutability: Option<Token![mut]>,
    and_token: Option<Token![&]>,
}

impl NodeCtx {
    fn new(input: &ItemFn) -> Result<Self, Error> {
        // Capture the identifier of the argument.
        let function_arg = input.sig.inputs.first().expect("Input has no inputs");
        let (pat, ty) = match function_arg {
            FnArg::Typed(function_arg) => (function_arg.pat.as_ref(), function_arg.ty.as_ref()),
            FnArg::Receiver(_) => {
                // Passed parameter is a `self`.
                let msg = "Input argument should be of type `ockam::Context`";
                return Err(Error::new_spanned(function_arg, msg));
            }
        };
        let ident = match pat {
            Pat::Ident(ident) => ident,
            _ => {
                let msg = format!("Expected an identifier, found `{}`", quote! {#pat});
                return Err(Error::new_spanned(pat, msg));
            }
        };
        let (mutability, and_token) = match ty {
            Type::Reference(ty) => (ty.mutability, Some(ty.and_token)),
            _ => (ident.mutability, None),
        };
        Ok(Self {
            pat: ident.clone(),
            ty: ty.clone(),
            mutability,
            and_token,
        })
    }

    fn node_test_checks(&self, input: &ItemFn) -> Result<(), Error> {
        self.arg_is_ctx()?;
        self.ctx_is_mut_ref(input)?;
        Ok(())
    }

    fn arg_is_ctx(&self) -> Result<(), Error> {
        // Verify that the type is `ockam::Context` (We only verify that the type is `Context`).
        // If it is some other context, there might be other compiler error, so that's fine.
        let parse_path = |path: &Path| match path.segments.last() {
            None => {
                let msg = "Input argument should be of type `ockam::Context`";
                Err(Error::new_spanned(&path, msg))
            }
            Some(seg) => {
                let ident = seg.ident.to_string();
                if ident != "Context" {
                    let path_ident = quote! {#path}.to_string().replace(' ', "");
                    let msg = format!("Expected `ockam::Context` found `{}`", &path_ident);
                    return Err(Error::new_spanned(&path, msg));
                }
                Ok(())
            }
        };
        match &self.ty {
            Type::Path(ty) => Ok(parse_path(&ty.path)?),
            Type::Reference(ty) => {
                if let Type::Path(TypePath { qself: _, path }) = &*ty.elem {
                    Ok(parse_path(path)?)
                } else {
                    let msg = format!("Unexpected argument type {:?}", &self.ty);
                    Err(Error::new_spanned(ty, msg))
                }
            }
            _ => {
                let msg = format!("Unexpected argument type {:?}", &self.ty);
                Err(Error::new_spanned(&self.ty, msg))
            }
        }
    }

    fn ctx_is_mut_ref(&self, input: &ItemFn) -> Result<(), Error> {
        if self.and_token.is_none() {
            let msg = "The context argument must be passed as reference";
            return Err(Error::new_spanned(input.sig.fn_token, msg));
        }
        if self.mutability.is_none() {
            let msg = "The context argument must be mutable";
            return Err(Error::new_spanned(input.sig.fn_token, msg));
        }
        Ok(())
    }
}

fn cleanup(mut input: ItemFn) -> Result<ItemFn, Error> {
    // Remove the arguments
    input.sig.inputs = Punctuated::new();
    // Remove the output
    input.sig.output = ReturnType::Default;
    // Remove async
    input.sig.asyncness = None;
    Ok(input)
}
