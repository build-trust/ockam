use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::meta::parser;
use syn::parse::Parser;
use syn::{Expr, ItemFn, ReturnType};

use crate::internals::attr::{parse_lit_into_path, Attr, BoolAttr};
use crate::internals::{ast, ast::FnVariable, check, ctx::Context, symbol::*};

pub(crate) fn expand(
    input_fn: ItemFn,
    attrs: &TokenStream,
) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Context::new();
    let cont = Container::from_ast(&ctx, &input_fn, attrs);
    ctx.check()?;
    Ok(output(cont))
}

fn output(cont: Container) -> TokenStream {
    let body = &cont.original_fn.block;
    let ret_type = cont.data.ret;
    let ockam_crate = cont.data.arguments.ockam_crate;

    // Get the ockam context variable identifier and mutability token extracted
    // from the function arguments, or sets them to their default values.
    let (ctx_ident, ctx_mut, ctx_path) = match &cont.data.ockam_ctx {
        None => (quote! {_ctx}, quote! {}, quote! {ockam::Context}),
        Some(ctx_var) => {
            let ident = &ctx_var.ident;
            let mutability = &ctx_var.mutability;
            let path = &ctx_var.path;
            (quote! {#ident}, quote! {#mutability}, quote! {#path})
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

    if !cont.data.arguments.no_main {
        // Assumes the target platform knows about main() functions
        quote! {
            fn main() #ret_type {
                use #ockam_crate::{NodeBuilder, Executor};

                let (#ctx_mut #ctx_ident, mut executor) = NodeBuilder::new().build() as (#ctx_path, Executor);
                executor.execute(async move #body)#err_handling
            }
        }
    } else {
        // Assumes you will be defining the ockam node inside your own entry point
        quote! {
            fn ockam_async_main() #ret_type {
                use #ockam_crate::{NodeBuilder, Executor};

                let (#ctx_mut #ctx_ident, mut executor) = NodeBuilder::with_access_control().build() as (#ctx_path, Executor);
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
    fn from_ast(ctx: &Context, input_fn: &'a ItemFn, args: &'a TokenStream) -> Self {
        let cont = Self {
            data: Data::from_ast(ctx, input_fn, args),
            original_fn: input_fn,
        };
        cont.check(ctx);
        cont
    }

    // The macro should not prevent the user from using an input function with the following features:
    //   - without ockam context, not using the ockam context, empty body: in any of these cases, the node
    //     will just run indefinitely, which is also OK.
    //   - non async: the user might want to run a dummy node without any async code.
    //   - multiple arguments: only the ockam context will be used. The other arguments will be ignored.
    //
    // Therefore, the macro tries to be as permissive as possible and only checks critical things that
    // would not compile.
    fn check(&self, ctx: &Context) {
        #[cfg(not(feature = "no_main"))]
        check::item_fn::ident_is_main(ctx, self.original_fn);
    }
}

struct Data<'a> {
    // Macro attributes.
    arguments: TestArguments,
    // The `ctx` variable data extracted from the input function arguments.
    // (e.g. from `ctx: &mut ockam::Context` it extracts `ctx`, `&` and `mut`).
    ockam_ctx: Option<FnVariable<'a>>,
    // The function's return type (e.g. `ockam::Result<()>`).
    ret: &'a ReturnType,
}

impl<'a> Data<'a> {
    fn from_ast(ctx: &Context, input_fn: &'a ItemFn, args: &TokenStream) -> Self {
        Self {
            arguments: TestArguments::from_ast(ctx, args),
            ockam_ctx: ast::ockam_context_variable_from_input_fn(ctx, input_fn),
            ret: &input_fn.sig.output,
        }
    }
}

struct TestArguments {
    ockam_crate: TokenStream,
    no_main: bool,
}

impl TestArguments {
    fn from_ast(ctx: &Context, args: &TokenStream) -> Self {
        let mut ockam_crate = Attr::none(ctx, OCKAM_CRATE);
        let mut no_main = BoolAttr::none(ctx, NO_MAIN);

        let p = parser(|meta| {
            if meta.path.is_ident(&OCKAM_CRATE) {
                let value_expr: Expr = meta.value()?.parse()?;
                if let Ok(path) = parse_lit_into_path(ctx, OCKAM_CRATE, &value_expr) {
                    let path = quote! { #path };
                    ockam_crate.set(&meta.path, path);
                };
                Ok(())
            } else if meta.path.is_ident(&NO_MAIN) {
                no_main.set_true(meta.path);
                Ok(())
            } else {
                ctx.error_spanned_by(
                    meta.path.clone(),
                    format!("unknown attribute `{}`", meta.path.into_token_stream()),
                );
                Ok(())
            }
        });
        p.parse(args.clone().into()).unwrap_or_default();

        Self {
            ockam_crate: ockam_crate.get().unwrap_or(quote! { ockam }),
            no_main: no_main.get(),
        }
    }
}
