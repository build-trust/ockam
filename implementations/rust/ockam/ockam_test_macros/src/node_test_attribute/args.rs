use proc_macro2::Span;

pub(crate) fn node_test(args: syn::AttributeArgs) -> Result<TestArgs, syn::Error> {
    let mut parsed_args = TestArgsBuilder::default();

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
                match ident.as_str() {
                    "timeout" => {
                        parsed_args.set_timeout(
                            namevalue.lit.clone(),
                            syn::spanned::Spanned::span(&namevalue.lit),
                        )?;
                    }
                    name => {
                        let msg = format!(
                            "Unknown attribute {} is specified; expected one of: {}",
                            name,
                            print_valid_test_args()
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
                let msg = match name.as_str() {
                    "timeout" => {
                        format!("The `{}` attribute requires an argument.", name)
                    }
                    name => {
                        format!(
                            "Unknown attribute {} is specified; expected one of: {}",
                            name,
                            print_valid_test_args()
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
    Ok(parsed_args.build())
}

const VALID_TEST_ARGS: &[&str] = &["timeout"];

fn print_valid_test_args() -> String {
    VALID_TEST_ARGS.join(".")
}

pub(crate) struct TestArgs {
    pub(crate) timeout_ms: usize,
}

impl TestArgs {
    const DEFAULT_TIMEOUT_MS: usize = 30000;
}

#[derive(Default)]
struct TestArgsBuilder {
    timeout_ms: Option<(usize, Span)>,
}

impl TestArgsBuilder {
    fn set_timeout(&mut self, timeout: syn::Lit, span: Span) -> Result<(), syn::Error> {
        if self.timeout_ms.is_some() {
            return Err(syn::Error::new(span, "`timeout` set multiple times."));
        }

        let timeout = parse_int(timeout, span, "timeout")?;
        if timeout == 0 {
            return Err(syn::Error::new(span, "`timeout` can't be 0."));
        }
        self.timeout_ms = Some((timeout, span));
        Ok(())
    }

    fn build(&self) -> TestArgs {
        let timeout_ms = match self.timeout_ms {
            None => TestArgs::DEFAULT_TIMEOUT_MS,
            Some(arg) => arg.0,
        };
        TestArgs { timeout_ms }
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
