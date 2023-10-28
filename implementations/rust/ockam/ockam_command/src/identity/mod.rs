mod create;
mod default;
mod delete;
mod list;
mod show;

pub use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::identity::default::DefaultCommand;
use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliState;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Identities
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
long_about = docs::about(LONG_ABOUT),
)]
pub struct IdentityCommand {
    #[command(subcommand)]
    subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    List(ListCommand),
    Default(DefaultCommand),
    Delete(DeleteCommand),
}

impl IdentityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            IdentitySubcommand::Create(c) => c.run(options),
            IdentitySubcommand::Show(c) => c.run(options),
            IdentitySubcommand::List(c) => c.run(options),
            IdentitySubcommand::Delete(c) => c.run(options),
            IdentitySubcommand::Default(c) => c.run(options),
        }
    }
}

/// If the required identity is the default identity but if it has not been initialized yet
/// then initialize it
pub fn initialize_identity_if_default(opts: &CommandGlobalOpts, name: &Option<String>) {
    let name = get_identity_name(&opts.state, name);
    if name == "default" && opts.state.identities.default().is_err() {
        create_default_identity(opts);
    }
}

/// Return the name if identity_name is Some otherwise return the name of the default identity
pub fn get_identity_name(cli_state: &CliState, identity_name: &Option<String>) -> String {
    identity_name
        .clone()
        .unwrap_or_else(|| get_default_identity_name(cli_state))
}

/// Return the name of the default identity
pub fn get_default_identity_name(cli_state: &CliState) -> String {
    cli_state
        .identities
        .default()
        .map(|i| i.name().to_string())
        .unwrap_or_else(|_| "default".to_string())
}

/// Create the default identity
pub fn create_default_identity(opts: &CommandGlobalOpts) {
    let default = "default";
    let create_command = CreateCommand::new(default.into(), None, None);
    create_command.run(opts.clone().set_quiet());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalArgs;
    use ockam_api::cli_state::StateItemTrait;

    #[test]
    fn test_initialize() {
        let state = CliState::test().unwrap();
        let opts = CommandGlobalOpts::new_for_test(GlobalArgs::default(), state);

        // on start-up there is no default identity
        assert!(opts.state.identities.default().is_err());

        // if no name is given then the default identity is initialized
        initialize_identity_if_default(&opts, &None);
        assert!(opts.state.identities.default().is_ok());

        // if "default" is given as a name the default identity is initialized
        opts.state.identities.default().unwrap().delete().unwrap();
        initialize_identity_if_default(&opts, &Some("default".into()));
        assert!(opts.state.identities.default().is_ok());

        // if the name of another identity is given then the default identity is not initialized
        opts.state.identities.default().unwrap().delete().unwrap();
        initialize_identity_if_default(&opts, &Some("other".into()));
        assert!(opts.state.identities.default().is_err());
    }
}
