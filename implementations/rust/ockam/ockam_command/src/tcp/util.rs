use miette::miette;

use crate::Result;

pub fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(miette!("an alias must not contain ':' characters"))?
    } else {
        Ok(arg.to_string())
    }
}
