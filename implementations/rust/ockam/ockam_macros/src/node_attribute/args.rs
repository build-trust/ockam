pub(crate) fn node(args: syn::AttributeArgs) -> Result<Args, syn::Error> {
    match args.first() {
        None => Ok(Args {}),
        Some(arg) => Err(syn::Error::new_spanned(
            arg,
            "This macro doesn't accept any argument",
        )),
    }
}

pub(crate) struct Args {}
