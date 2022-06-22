use clap::Args;
use std::str::FromStr;

/// A plugin command type that can
#[derive(Clone, Debug, Args)]
pub struct AddonCommand {
    /// The operation to perform
    #[clap(possible_values = vec!["create", "delete", "show", "list"])]
    operation: String,
    /// The name of the addon command.  Its full name must be
    /// `ockam-<scope>-<name>`, so for example:
    /// `ockam-transport-create-tcp-inlet`
    addon_name: Option<String>,
    /// Everything else given to this command will be ignored by the
    /// ockam CLI but forwarded to the plugin command
    #[clap(hide = true)]
    _proxy: Vec<String>,
}

impl FromStr for AddonCommand {
    // Errors are so meaningless in this process, we never emit any
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, &'static str> {
        let mut s: Vec<_> = s.split_whitespace().collect();
        Ok(Self {
            operation: s.remove(0).into(),
            addon_name: Some(s.remove(0).into()),
            _proxy: s.into_iter().map(Into::into).collect(),
        })
    }
}

impl AddonCommand {
    /// Print the inner help text for this particular addon
    pub fn get_help(&self) -> String {
        todo!()
    }
}
