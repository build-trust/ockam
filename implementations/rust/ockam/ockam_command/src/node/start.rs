use clap::Args;

use ockam::TcpTransport;

use crate::node::show::print_query_status;
use crate::node::util::{default_node_name, spawn_node};
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};

/// Start a node
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct StartCommand {
    /// Name of the node.
    node_name: Option<String>,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
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
    let node_name = &match cmd.node_name {
        Some(name) => name,
        None => default_node_name(&opts),
    };

    let node_state = opts.state.nodes.get(node_name)?;
    node_state.kill_process(false)?;
    let node_setup = node_state.setup()?;

    // Restart node
    spawn_node(
        &opts,
        node_setup.verbose, // Previously user-chosen verbosity level
        node_name,          // The selected node name
        &node_setup.default_tcp_listener()?.addr.to_string(), // The selected node api address
        None,               // No project information available
        None,               // No invitation code available
    )?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    print_query_status(&mut rpc, node_name, true).await?;

    Ok(())
}
