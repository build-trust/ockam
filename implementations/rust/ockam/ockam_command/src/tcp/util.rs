use crate::Result;
use anyhow::anyhow;

pub fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(anyhow!("an alias must not contain ':' characters").into())
    } else {
        Ok(arg.to_string())
    }
}
