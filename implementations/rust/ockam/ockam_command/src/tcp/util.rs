use crate::Result;
use miette::miette;

pub fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(miette!("an alias must not contain ':' characters").into())
    } else {
        Ok(arg.to_string())
    }
}
