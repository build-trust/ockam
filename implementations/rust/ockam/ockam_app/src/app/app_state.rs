use std::sync::Arc;

use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_command::{CommandGlobalOpts, GlobalArgs, Terminal};

#[derive(Clone)]
pub struct AppState {
    pub context: Arc<Context>,
    pub global_args: GlobalArgs,
    pub state: CliState,
}

impl From<AppState> for CommandGlobalOpts {
    fn from(app_state: AppState) -> CommandGlobalOpts {
        app_state.options()
    }
}

impl AppState {
    pub fn new(context: Arc<Context>, options: CommandGlobalOpts) -> Self {
        Self {
            context,
            global_args: options.global_args,
            state: options.state,
        }
    }

    pub fn options(&self) -> CommandGlobalOpts {
        CommandGlobalOpts {
            global_args: self.global_args.clone(),
            state: self.state.clone(),
            terminal: Terminal::default(),
        }
    }
}
