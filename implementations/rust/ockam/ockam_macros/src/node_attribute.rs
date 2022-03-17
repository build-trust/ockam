use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{AttributeArgs, ItemFn, Meta::Path, NestedMeta, ReturnType};

use crate::internals::{ast, ast::FnVariable, attr::BoolAttr, check, ctx::Context, symbol::*};

pub(crate) fn expand(
    input_fn: ItemFn,
    attrs: AttributeArgs,
) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Context::new();
    let cont = Container::from_ast(&ctx, &input_fn, &attrs);
    ctx.check()?;
    Ok(output(cont))
}

fn output(cont: Container) -> TokenStream {
    let body = &cont.original_fn.block;
    let ret_type = cont.data.ret;

    // Get the ockam context variable identifier and mutability token extracted
    // from the function arguments, or sets them to their default values.
    let (ctx_ident, ctx_mut) = match &cont.data.ockam_ctx {
        None => (quote! {_ctx}, quote! {}),
        Some(ctx_var) => {
            let ident = &ctx_var.ident;
            let mutability = &ctx_var.mutability;
            (quote! {#ident}, quote! {#mutability})
        }
    };

    // Handles error if inner function returns Result, unwraps it otherwise.
    let err_handling = if matches!(ret_type, ReturnType::Default) {
        quote! {.unwrap();}
    } else {
        #[cfg(feature = "std")]
        quote! {?}

        // For now the executor's `Executor::execute` for std returns `Result<F::Output>`
        // while for no_std it returns `Result<()>` and always returns `Ok(())` or panics.
        // So while it makes sense using the `?` operator in std in no_std just returning Ok(())
        // would be enough(since execute already hides the return type) but we are letting the
        // `Executor::execute` return bubble up to keep the code simpler.
        // Note: This also means that for no_std `main` return type can only be `Result<()>` or nothing.
        #[cfg(not(feature = "std"))]
        quote! {}
    };

    if !cont.data.attrs.no_main {
        // Assumes the target platform knows about main() functions
        quote! {
            fn main() #ret_type {
                let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (ockam::Context, ockam::Executor);
                executor.execute(async move #body)#err_handling
            }
        }
    } else {
        // Assumes you will be defining the ockam node inside your own entry point
        quote! {
            fn ockam_async_main() #ret_type {
                let (#ctx_mut #ctx_ident, mut executor) = ockam::start_node() as (ockam::Context, ockam::Executor);
                executor.execute(async move #body)#err_handling
            }
            // TODO: safe way to print the error before panicking?
            ockam_async_main().unwrap();
        }
    }
}

struct Container<'a> {
    // Macro data.
    data: Data<'a>,
    // Original function.
    original_fn: &'a ItemFn,
}

impl<'a> Container<'a> {
    pub fn from_ast(ctx: &Context, input_fn: &'a ItemFn, attrs: &'a AttributeArgs) -> Self {
        let cont = Self {
            data: Data::from_ast(ctx, input_fn, attrs),
            original_fn: input_fn,
        };
        cont.check(ctx);
        cont
    }

    /// The macro should not prevent the user from using an input function with the following features:
    ///   - without ockam context, not using the ockam context, empty body: in any of these cases, the node
    ///     will just run indefinitely, which is also OK.
    ///   - non async: the user might want to run a dummy node without any async code.
    ///   - multiple arguments: only the ockam context will be used. The other arguments will be ignored.
    ///
    /// Therefore, the macro tries to be as permissive as possible and only checks critical things that
    /// would not compile.
    fn check(&self, ctx: &Context) {
        #[cfg(not(feature = "no_main"))]
        check::item_fn::ident_is_main(ctx, self.original_fn);

        // TODO: removed checks -- too aggressive and not really mandatory, without them the output is still functional.
        // check::item_fn::is_async(ctx, self.original_fn); // if the code is not async it should still run.
        // check::item_fn::has_ockam_ctx_arg(ctx, &self.data.ockam_ctx); // if the user wants to
        // check::item_fn::body_is_not_empty(self.original);
        // check::item_fn::has_one_arg(self.original);
        // check::item_fn::ockam_context_is_used(self.original, self.data.ockam_ctx);
    }
}

struct Data<'a> {
    // Macro attributes.
    attrs: Attributes,
    // The `ctx` variable data extracted from the input function arguments.
    // (e.g. from `ctx: &mut ockam::Context` it extracts `ctx`, `&` and `mut`).
    ockam_ctx: Option<FnVariable<'a>>,
    // The function's return type (e.g. `ockam::Result<()>`).
    ret: &'a ReturnType,
}

impl<'a> Data<'a> {
    fn from_ast(ctx: &Context, input_fn: &'a ItemFn, attrs: &AttributeArgs) -> Self {
        Self {
            attrs: Attributes::from_ast(ctx, attrs),
            ockam_ctx: ast::ockam_context_variable_from_input_fn(ctx, input_fn),
            ret: &input_fn.sig.output,
        }
    }
}

struct Attributes {
    no_main: bool,
}

impl Attributes {
    fn from_ast(ctx: &Context, attrs: &AttributeArgs) -> Self {
        let mut no_main = BoolAttr::none(ctx, NO_MAIN);
        for attr in attrs {
            match attr {
                // Parse `#[ockam::node(no_main)]`
                NestedMeta::Meta(Path(p)) if p == NO_MAIN => {
                    no_main.set_true(p);
                }
                NestedMeta::Meta(m) => {
                    let path = m.path().into_token_stream().to_string().replace(' ', "");
                    ctx.error_spanned_by(m.path(), format!("unknown attribute `{}`", path));
                }
                NestedMeta::Lit(lit) => {
                    ctx.error_spanned_by(lit, "unexpected literal in attribute");
                }
            }
        }
        Self {
            no_main: no_main.get(),
        }
    }
}
