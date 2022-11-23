use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    AttributeArgs, ItemFn,
    Meta::{NameValue, Path},
    NestedMeta, ReturnType,
};

use crate::internals::attr::{parse_lit_into_path, Attr, BoolAttr};
use crate::internals::{ast, ast::FnVariable, check, ctx::Context, symbol::*};

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
    let ockam_crate = cont.data.attrs.ockam_crate;
    let incoming_access_control = cont.data.attrs.incoming_access_control;
    let outgoing_access_control = cont.data.attrs.outgoing_access_control;

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

    if !cont.data.attrs.no_main {
        // Assumes the target platform knows about main() functions
        quote! {
            fn main() #ret_type {
                use #ockam_crate::{NodeBuilder, Executor};

                let (#ctx_mut #ctx_ident, mut executor) = NodeBuilder::with_access_control(
                    #ockam_crate::compat::sync::Arc::new(#incoming_access_control),
                    #ockam_crate::compat::sync::Arc::new(#outgoing_access_control)
                ).build() as (#ctx_path, Executor);
                executor.execute(async move #body)#err_handling
            }
        }
    } else {
        // Assumes you will be defining the ockam node inside your own entry point
        quote! {
            fn ockam_async_main() #ret_type {
                use #ockam_crate::{NodeBuilder, Executor};

                let (#ctx_mut #ctx_ident, mut executor) = NodeBuilder::with_access_control(
                    #ockam_crate::compat::sync::Arc::new(#incoming_access_control),
                    #ockam_crate::compat::sync::Arc::new(#outgoing_access_control)
                ).build() as (#ctx_path, Executor);
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
    fn from_ast(ctx: &Context, input_fn: &'a ItemFn, attrs: &'a AttributeArgs) -> Self {
        let cont = Self {
            data: Data::from_ast(ctx, input_fn, attrs),
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
    incoming_access_control: TokenStream,
    outgoing_access_control: TokenStream,
    ockam_crate: TokenStream,
    no_main: bool,
}

impl Attributes {
    fn from_ast(ctx: &Context, attrs: &AttributeArgs) -> Self {
        let mut incoming_access_control = Attr::none(ctx, INCOMING_ACCESS_CONTROL);
        let mut outgoing_access_control = Attr::none(ctx, OUTGOING_ACCESS_CONTROL);
        let mut ockam_crate = Attr::none(ctx, OCKAM_CRATE);
        let mut no_main = BoolAttr::none(ctx, NO_MAIN);
        for attr in attrs {
            match attr {
                // Parse `#[ockam::test(incoming = "LocalOriginOnly")]`
                NestedMeta::Meta(NameValue(nv)) if nv.path == INCOMING_ACCESS_CONTROL => {
                    if let Ok(path) = parse_lit_into_path(ctx, INCOMING_ACCESS_CONTROL, &nv.lit) {
                        incoming_access_control.set(&nv.path, quote! { #path });
                    }
                }
                // Parse `#[ockam::test(outgoing = "LocalDestinationOnly")]`
                NestedMeta::Meta(NameValue(nv)) if nv.path == OUTGOING_ACCESS_CONTROL => {
                    if let Ok(path) = parse_lit_into_path(ctx, OUTGOING_ACCESS_CONTROL, &nv.lit) {
                        outgoing_access_control.set(&nv.path, quote! { #path });
                    }
                }
                // Parse `#[ockam::test(crate = "ockam")]`
                NestedMeta::Meta(NameValue(nv)) if nv.path == OCKAM_CRATE => {
                    if let Ok(path) = parse_lit_into_path(ctx, OCKAM_CRATE, &nv.lit) {
                        ockam_crate.set(&nv.path, quote! { #path });
                    }
                }
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
            incoming_access_control: incoming_access_control
                .get()
                .unwrap_or(quote! { ockam::access_control::DenyAll }),
            outgoing_access_control: outgoing_access_control
                .get()
                .unwrap_or(quote! { ockam::access_control::DenyAll }),
            ockam_crate: ockam_crate.get().unwrap_or(quote! { ockam }),
            no_main: no_main.get(),
        }
    }
}
