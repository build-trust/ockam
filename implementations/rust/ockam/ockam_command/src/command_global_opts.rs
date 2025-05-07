use console::Term;
use std::sync::Arc;
use tracing::debug;

use crate::version::Version;
use crate::GlobalArgs;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::CliState;

/// This struct contains the main structs used to implement commands:
///
///  - The arguments applicable to all commands
///  - The CliState, which provides an access to both the local state and interfaces to remote nodes
///  - The terminal used to output the command results
///
#[derive(Clone, Debug)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    // TODO: This is not the place for it. We could propagate it more granularly and even avoid
    //  creating it for some commands.
    pub state: Arc<CliState>,
    pub terminal: Terminal<TerminalStream<Term>>,
}

impl CommandGlobalOpts {
    /// Create new CommandGlobalOpts:
    ///
    ///  - Instantiate logging + tracing
    ///  - Initialize the CliState
    ///  - Get the runtime
    ///
    pub fn new(
        global_args: GlobalArgs,
        state: Arc<CliState>,
        terminal: Terminal<TerminalStream<Term>>,
    ) -> Self {
        Self {
            global_args: global_args.clone(),
            state,
            terminal,
        }
    }

    /// Log the inputs and configurations used to execute the command
    pub(crate) fn log_inputs(&self, arguments: &[String], cmd_name: &str) {
        debug!("Arguments: {}", arguments.join(" "));
        debug!("Global arguments: {:#?}", &self.global_args);
        debug!("Command: {:#?}", &cmd_name);
        debug!("Version: {}", Version::new().no_color());
    }

    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.global_args = clone.global_args.set_quiet();
        clone.terminal = clone.terminal.set_quiet();
        clone
    }
}
