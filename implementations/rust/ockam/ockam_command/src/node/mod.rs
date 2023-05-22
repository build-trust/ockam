use clap::{Args, Subcommand};

use colorful::Colorful;
pub(crate) use create::CreateCommand;
use default::DefaultCommand;
use delete::DeleteCommand;
use list::ListCommand;
use logs::LogCommand;
use ockam_api::cli_state::{CliState, StateDirTrait};
use show::ShowCommand;
use start::StartCommand;
use stop::StopCommand;

use crate::{docs, fmt_log, terminal::OckamColor, CommandGlobalOpts, PARSER_LOGS};

mod create;
mod default;
mod delete;
mod list;
mod logs;
mod show;
mod start;
mod stop;
pub mod util;
pub use create::*;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage nodes
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct NodeCommand {
    #[command(subcommand)]
    pub subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    #[command(display_order = 800)]
    Create(Box<CreateCommand>),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
    #[command(display_order = 800)]
    List(ListCommand),
    #[command(display_order = 800)]
    Logs(LogCommand),
    Show(ShowCommand),
    #[command(display_order = 800)]
    Start(StartCommand),
    #[command(display_order = 800)]
    Stop(StopCommand),
    #[command(display_order = 800)]
    Default(DefaultCommand),
}

impl NodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            NodeSubcommand::Create(c) => c.run(options),
            NodeSubcommand::Delete(c) => c.run(options),
            NodeSubcommand::List(c) => c.run(options),
            NodeSubcommand::Show(c) => c.run(options),
            NodeSubcommand::Start(c) => c.run(options),
            NodeSubcommand::Stop(c) => c.run(options),
            NodeSubcommand::Logs(c) => c.run(options),
            NodeSubcommand::Default(c) => c.run(options),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct NodeOpts {
    /// Override the default API node
    #[arg(global = true, id = "node", value_name = "NODE", short, long)]
    pub api_node: Option<String>,
}

/// If the required node name is the default node but that node has not been initialized yet
/// then initialize it
pub fn initialize_node(opts: &CommandGlobalOpts, node_name: &Option<String>) {
    let node_name = get_node_name(&opts.state, node_name);
    if node_name == "default" && opts.state.nodes.default().is_err() {
        spawn_default_node(opts)
    }
}

/// Return the node_name if Some otherwise return the default node name
pub fn get_node_name(cli_state: &CliState, node_name: &Option<String>) -> String {
    node_name
        .clone()
        .unwrap_or_else(|| get_default_node_name(cli_state))
}

/// Return the default node name
pub fn get_default_node_name(cli_state: &CliState) -> String {
    cli_state
        .nodes
        .default()
        .map(|n| n.name().to_string())
        .unwrap_or_else(|_| "default".to_string())
}

/// Start the default node
fn spawn_default_node(opts: &CommandGlobalOpts) {
    let mut create_command = CreateCommand::default();

    let default = "default";
    create_command.node_name = default.into();
    let mut quiet_opts = opts.clone();
    quiet_opts.set_quiet();
    create_command.run(quiet_opts);

    if let Ok(mut logs) = PARSER_LOGS.lock() {
        logs.push(fmt_log!("No default node was found."));
        logs.push(fmt_log!(
            "Created default node, {}",
            default.color(OckamColor::PrimaryResource.color())
        ));
    }
}
