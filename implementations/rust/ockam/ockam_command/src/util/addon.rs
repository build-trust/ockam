// This abstraction is currently not used because of the way that we
// structure the main clap parser and the difficulties it brings with
// generating meaningful but dynamic help pages.
//
// When we want to support add-on commands we will have to dig this
// structure out again and likely re-build the way we currently parse
// commandline sections and generate the help pages.

use clap::Args;
use std::str::FromStr;

/// A plugin command type that can
#[derive(Clone, Debug, Args)]
pub struct AddonCommand {
    /// Operation to perform
    #[clap(value_parser(["create", "delete", "show", "list"]))]
    operation: String,
    /// Add-on subcommand to call
    // Its full name must be `ockam-<scope>-<name>`, so for example:
    // `ockam-transport-create-tcp-inlet`
    addon_name: Option<String>,
    /// Other options passed into the add-on command
    options: Vec<String>,
}

impl FromStr for AddonCommand {
    // Errors are so meaningless in this process, we never emit any
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, &'static str> {
        let mut s: Vec<_> = s.split_whitespace().collect();
        Ok(Self {
            operation: s.remove(0).into(),
            addon_name: Some(s.remove(0).into()),
            options: s.into_iter().map(Into::into).collect(),
        })
    }
}
