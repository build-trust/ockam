use syn::{AttributeArgs, Meta, NestedMeta};

pub(crate) fn node(args: AttributeArgs) -> Result<Args, syn::Error> {
    match args.first() {
        None => Ok(Args { no_main: false }),
        Some(NestedMeta::Meta(Meta::Path(x))) if x.is_ident("no_main") => {
            Ok(Args { no_main: true })
        }
        Some(arg) => Err(syn::Error::new_spanned(
            arg,
            "`ockam::node` does not support this syntax",
        )),
    }
}

pub(crate) struct Args {
    pub(crate) no_main: bool,
}
