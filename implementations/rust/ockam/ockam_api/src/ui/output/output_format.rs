use super::Output;
use crate::Result;
use clap::ValueEnum;

/// There are 2 available formats:
///
///  - Plain formats a user readable string
///  - Json returns some prettified JSON
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
}

impl OutputFormat {
    /// Print a value on the console for any value having a textual Output and a JSON
    /// representation via serde
    pub fn println_value<T>(&self, t: &T) -> Result<()>
    where
        T: Output + serde::Serialize,
    {
        let output = match self {
            OutputFormat::Plain => t.single()?,
            OutputFormat::Json => {
                serde_json::to_string_pretty(t).map_err(crate::ParseError::SerdeJson)?
            }
        };
        println!("{output}");
        Ok(())
    }
}
