use miette::IntoDiagnostic;
use ockam_core::errcode::{Kind, Origin};
use serde::Deserialize;

use crate::run::parser::Variables;

pub trait ConfigParser<'de>: Deserialize<'de> {
    /// Parse variables section and resolve them
    fn resolve(contents: &str) -> miette::Result<String> {
        Variables::resolve(contents)
    }
    /// Parses a given yaml configuration
    fn parse(contents: &'de str) -> miette::Result<Self> {
        serde_yaml::from_str(contents)
            .map_err(|e| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!(
                        "could not parse the node configuration file: {e:?}\n\n{}",
                        contents
                    ),
                )
            })
            .into_diagnostic()
    }
}
