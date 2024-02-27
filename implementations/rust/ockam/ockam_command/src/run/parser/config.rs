use crate::run::parser::Variables;
use miette::IntoDiagnostic;
use serde::Deserialize;

pub trait ConfigParser<'de>: Deserialize<'de> {
    /// Parse variables section and resolve them
    fn resolve(contents: &str) -> miette::Result<String> {
        Variables::resolve(contents)
    }
    /// Parses a given yaml configuration
    fn parse(contents: &'de str) -> miette::Result<Self> {
        serde_yaml::from_str(contents).into_diagnostic()
    }
}
