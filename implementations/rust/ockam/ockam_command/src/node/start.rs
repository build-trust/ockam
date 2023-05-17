use clap::Args;

use ockam::TcpTransport;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

use crate::node::get_node_name;
use crate::node::show::print_query_status;
use crate::node::util::{check_default, spawn_node};
use crate::util::{node_rpc, RpcBuilder};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/start/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/start/after_long_help.txt");

/// Start a node that was previously stopped
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StartCommand {
    /// Name of the node.
    #[arg()]
    node_name: Option<String>,

    #[arg(long, default_value = "false")]
    aws_kms: bool,

    #[arg(long, default_value = "false")]
    force: bool,
}

impl StartCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, cmd.node_name.clone())?;

    let node_state = opts.state.nodes.get(&node_name)?;
    // Check if node is already running
    if node_state.is_running() && !cmd.force {
        println!(
            "Restart aborted, node: {} already running",
            node_state.name()
        );
        return Ok(());
    }
    node_state.kill_process(false)?;
    let node_setup = node_state.config().setup();

    // Restart node
    spawn_node(
        &opts,
        node_setup.verbose, // Previously user-chosen verbosity level
        &node_name,         // The selected node name
        &node_setup.default_tcp_listener()?.addr.to_string(), // The selected node api address
        None,               // No project information available
        None,               // No trusted identities
        None,               // "
        None,               // "
        None,               // Launch config
        None,               // Authoritiy Identity
        None,               // Credential
        None,               // Trust Context
        None,               // Project Name
    )?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, &node_name).tcp(&tcp)?.build();
    let is_default = check_default(&opts, &node_name);
    print_query_status(&mut rpc, &node_name, true, is_default).await?;

    Ok(())
}
