use proc_macro2::Span;

const VALID_ARGS: &[&str] = &[];
const VALID_TEST_ARGS: &[&str] = &["timeout"];

pub(crate) fn parse(args: syn::AttributeArgs, is_test: bool) -> Result<Args, syn::Error> {
    let mut parsed_args = ArgsBuilder::default();

    for arg in args {
        match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(namevalue)) => {
                let ident = namevalue
                    .path
                    .get_ident()
                    .ok_or_else(|| {
                        syn::Error::new_spanned(&namevalue, "Must have specified ident")
                    })?
                    .to_string()
                    .to_lowercase();
                match (ident.as_str(), is_test) {
                    ("timeout", true) => {
                        parsed_args.set_timeout(
                            namevalue.lit.clone(),
                            syn::spanned::Spanned::span(&namevalue.lit),
                        )?;
                    }
                    (name, _) => {
                        let msg = format!(
                            "Unknown attribute {} is specified; expected one of: {}",
                            name,
                            print_valid_arts(is_test)
                        );
                        return Err(syn::Error::new_spanned(namevalue, msg));
                    }
                }
            }
            syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                let name = path
                    .get_ident()
                    .ok_or_else(|| syn::Error::new_spanned(&path, "Must have specified ident"))?
                    .to_string()
                    .to_lowercase();
                let msg = match (name.as_str(), is_test) {
                    ("timeout", true) => {
                        format!("The `{}` attribute requires an argument.", name)
                    }
                    (name, is_test) => {
                        format!(
                            "Unknown attribute {} is specified; expected one of: {}",
                            name,
                            print_valid_arts(is_test)
                        )
                    }
                };
                return Err(syn::Error::new_spanned(path, msg));
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "Unknown attribute inside the macro",
                ));
            }
        }
    }
    parsed_args.build()
}

fn print_valid_arts(is_test: bool) -> String {
    let args = if is_test { VALID_TEST_ARGS } else { VALID_ARGS };
    args.join(".")
}

pub(crate) struct Args {
    pub(crate) timeout_ms: Option<usize>,
}

#[derive(Default)]
struct ArgsBuilder {
    timeout_ms: Option<(usize, Span)>,
}

impl ArgsBuilder {
    fn set_timeout(&mut self, timeout: syn::Lit, span: Span) -> Result<(), syn::Error> {
        if self.timeout_ms.is_some() {
            return Err(syn::Error::new(span, "`timeout` set multiple times."));
        }

        let timeout = parse_int(timeout, span, "timeout")?;
        if timeout == 0 {
            return Err(syn::Error::new(span, "`timeout` may not be 0."));
        }
        self.timeout_ms = Some((timeout, span));
        Ok(())
    }

    fn build(&self) -> Result<Args, syn::Error> {
        Ok(Args {
            timeout_ms: self.timeout_ms.map(|x| x.0),
        })
    }
}

fn parse_int(int: syn::Lit, span: Span, field: &str) -> Result<usize, syn::Error> {
    match int {
        syn::Lit::Int(lit) => match lit.base10_parse::<usize>() {
            Ok(value) => Ok(value),
            Err(e) => Err(syn::Error::new(
                span,
                format!("Failed to parse value of `{}` as integer: {}", field, e),
            )),
        },
        _ => Err(syn::Error::new(
            span,
            format!("Failed to parse value of `{}` as integer.", field),
        )),
    }
}
