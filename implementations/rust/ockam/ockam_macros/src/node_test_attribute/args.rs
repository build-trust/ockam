use proc_macro2::Span;
use quote::quote;

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
                    "crate" => {
                        parsed_args.set_crate(
                            namevalue.lit.clone(),
                            syn::spanned::Spanned::span(&namevalue.lit),
                        )?;
                    }
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
                    "crate" | "timeout" => {
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

const VALID_TEST_ARGS: &[&str] = &["crate", "timeout"];

fn print_valid_test_args() -> String {
    VALID_TEST_ARGS.join(",")
}

pub(crate) struct TestArgs {
    pub(crate) ockam_crate: proc_macro2::TokenStream,
    pub(crate) timeout_ms: usize,
}

#[derive(Default)]
struct TestArgsBuilder {
    ockam_crate: Option<(syn::Path, Span)>,
    timeout_ms: Option<(usize, Span)>,
}

impl TestArgsBuilder {
    fn set_crate(&mut self, ockam_crate: syn::Lit, span: Span) -> Result<(), syn::Error> {
        if self.ockam_crate.is_some() {
            return Err(syn::Error::new(span, "`crate` set multiple times."));
        }
        let ockam_crate_value = parse_string(&ockam_crate, span, "crate")?;
        if ockam_crate_value.is_empty() {
            return Err(syn::Error::new(span, "`crate` can't be empty."));
        }
        let ockam_crate_path = syn::parse2(syn::parse_str(&ockam_crate_value)?)?;
        self.ockam_crate = Some((ockam_crate_path, span));
        Ok(())
    }

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

    fn build(self) -> TestArgs {
        let ockam_crate = match self.ockam_crate {
            None => quote! { ockam_node },
            Some((path, _)) => quote! { #path },
        };
        let timeout_ms = match self.timeout_ms {
            None => 30000,
            Some(arg) => arg.0,
        };
        TestArgs {
            ockam_crate,
            timeout_ms,
        }
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

fn parse_string(int: &syn::Lit, span: Span, field: &str) -> Result<String, syn::Error> {
    match int {
        syn::Lit::Str(s) => Ok(s.value()),
        syn::Lit::Verbatim(s) => Ok(s.to_string()),
        _ => Err(syn::Error::new(
            span,
            format!("Failed to parse value of `{}` as string.", field),
        )),
    }
}
