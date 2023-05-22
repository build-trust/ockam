mod create;
mod default;
mod delete;
mod list;
mod show;

use colorful::Colorful;
pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::identity::default::DefaultCommand;
use crate::terminal::OckamColor;
use crate::{docs, fmt_log, fmt_ok, CommandGlobalOpts, PARSER_LOGS};
use clap::{Args, Subcommand};
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliState;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage identities
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
pub fn initialize_identity(opts: &CommandGlobalOpts, name: &Option<String>) {
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
fn create_default_identity(opts: &CommandGlobalOpts) {
    let default = "default";
    let create_command = CreateCommand::new(default.into(), None);
    let mut quiet_opts = opts.clone();
    quiet_opts.set_quiet();
    create_command.run(quiet_opts);

    if let Ok(mut logs) = PARSER_LOGS.lock() {
        logs.push(fmt_log!("No default identity was found."));
        logs.push(fmt_ok!(
            "Creating default identity {}",
            default.color(OckamColor::PrimaryResource.color())
        ));
        logs.push(fmt_log!(
            "Setting identity {} as default for local operations...\n",
            default.color(OckamColor::PrimaryResource.color())
        ));
    }
}
