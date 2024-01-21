use crate::Result;
use miette::miette;

pub fn relay_name_parser(arg: &str) -> Result<String> {
    if arg.starts_with("forward_to_") {
        Ok(arg.to_string())
    } else {
        Err(miette!(
            "The relay name must be prefixed with 'forward_to_'"
        ))?
    }
}
