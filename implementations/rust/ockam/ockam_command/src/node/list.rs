use crate::util::{api, exitcode, node_rpc, RpcBuilder};
use crate::{help, node::show::print_query_status, node::HELP_DETAIL, CommandGlobalOpts};
use anyhow::{anyhow, Context as _};
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::base::NodeStatus;
use std::time::Duration;

/// List Nodes
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, _cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    // Before printing node states we verify them.
    // We send a QueryStatus request to every node on
    // record. If the response yields a different pid to the
    // one in config, we update the pid stored in the config.
    // This should only happen if the node has failed in the past,
    // and has been restarted by something that is not this CLI.
    let node_names: Vec<_> = {
        let nodes_states = opts.state.nodes.list()?;
        if nodes_states.is_empty() {
            return Err(crate::Error::new(
                exitcode::IOERR,
                anyhow!("No nodes registered on this system!"),
            ));
        }
        nodes_states.iter().map(|s| s.config.name.clone()).collect()
    };
    let tcp = TcpTransport::create(&ctx).await?;
    verify_pids(&ctx, &opts, &tcp, &node_names).await?;

    // Print node states
    for node_name in &node_names {
        let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
        print_query_status(&mut rpc, node_name, false).await?;
    }

    Ok(())
}

/// Update the persisted configuration data with the pids
/// responded by nodes.
async fn verify_pids(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    nodes: &Vec<String>,
) -> crate::Result<()> {
    for node_name in nodes {
        if let Ok(node_state) = opts.state.nodes.get(node_name) {
            let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp)?.build();
            if rpc
                .request_with_timeout(api::query_status(), Duration::from_millis(200))
                .await
                .is_ok()
            {
                let resp = rpc.parse_response::<NodeStatus>()?;
                if node_state.pid()? != Some(resp.pid) {
                    node_state
                        .set_pid(resp.pid)
                        .context("Failed to update pid for node {node_name}")?;
                }
            }
        }
    }
    Ok(())
}
