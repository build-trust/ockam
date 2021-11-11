use quote::quote;
use syn::{
    punctuated::Punctuated, Error, FnArg, ItemFn, Pat, PatIdent, Path, ReturnType, Token, Type,
    TypePath,
};

pub(crate) fn node(input: ItemFn) -> Result<(ItemFn, NodeReturn, NodeCtx), Error> {
    let ret = NodeReturn::new(&input)?;
    ret.node_checks(&input)?;

    let ctx = NodeCtx::new(&input)?;
    ctx.node_checks(&input)?;

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

    fn node_checks(&self, input: &ItemFn) -> Result<(), Error> {
        #[cfg(not(feature = "no_main"))]
        self.fn_name_is_main(input)?;
        self.fn_is_async(input)?;
        self.body_is_not_empty(input)?;
        self.has_one_arg(input)?;
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

    #[cfg(not(feature = "no_main"))]
    fn fn_name_is_main(&self, input: &ItemFn) -> Result<(), Error> {
        if input.sig.ident != "main" {
            let msg = "The function name must be `main`";
            return Err(Error::new_spanned(input.sig.fn_token, msg));
        }
        Ok(())
    }

    fn body_is_not_empty(&self, input: &ItemFn) -> Result<(), Error> {
        // Function body cannot be empty (Special case of unused `context`).
        if input.block.stmts.is_empty() {
            let msg = "Function body Cannot be Empty.";
            return Err(Error::new_spanned(&input.sig.ident, msg));
        }
        Ok(())
    }
}

pub(crate) struct NodeCtx {
    pub(crate) pat: PatIdent,
    ty: Type,
    pub(crate) path: TypePath,
    pub(crate) mutability: Option<Token![mut]>,
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
        let mutability = match ty {
            Type::Reference(ty) => ty.mutability,
            _ => ident.mutability,
        };
        Ok(Self {
            pat: ident.clone(),
            ty: ty.clone(),
            path: Self::path(ty)?,
            mutability,
        })
    }

    fn path(ty: &Type) -> Result<TypePath, Error> {
        match ty {
            Type::Path(type_path) => Ok(type_path.clone()),
            Type::Reference(type_ref) => {
                if let Type::Path(type_path) = &*type_ref.elem {
                    Ok(type_path.clone())
                } else {
                    let msg = format!("Unexpected argument type {:?}", ty);
                    Err(Error::new_spanned(ty, msg))
                }
            }
            _ => {
                let msg = format!("Unexpected argument type {:?}", ty);
                Err(Error::new_spanned(ty, msg))
            }
        }
    }

    fn node_checks(&self, input: &ItemFn) -> Result<(), Error> {
        self.arg_is_ctx()?;
        self.ctx_is_used(input)?;
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

    fn ctx_is_used(&self, input: &ItemFn) -> Result<(), Error> {
        let mut ctx_used = false;
        for st in &input.block.stmts {
            let stmt_str = quote! {#st}.to_string().replace(' ', "");
            if stmt_str.contains(&self.pat.ident.to_string()) {
                ctx_used = true;
            }
        }
        if !ctx_used {
            let msg = format!(
                "Unused `{}`. Passed `ockam::Context` should be used.",
                &self.pat.ident.to_string()
            );
            return Err(Error::new_spanned(&self.pat.ident, msg));
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
