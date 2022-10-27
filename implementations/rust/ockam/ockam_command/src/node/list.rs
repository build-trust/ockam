use crate::util::{exitcode, node_rpc, verify_pids, RpcBuilder};
use crate::{help, node::show::print_query_status, node::HELP_DETAIL, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use ockam::TcpTransport;

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
    ctx: ockam::Context,
    (opts, _cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let cfg = &opts.config;

    // Before printing node states we verify them.
    // We send a QueryStatus request to every node on
    // record. If the response yields a different pid to the
    // one in config, we update the pid stored in the config.
    // This should only happen if the node has failed in the past,
    // and has been restarted by something that is not this CLI.
    let node_names: Vec<_> = {
        let inner = cfg.inner();
        if inner.nodes.is_empty() {
            return Err(crate::Error::new(
                exitcode::IOERR,
                anyhow!("No nodes registered on this system!"),
            ));
        }
        inner.nodes.iter().map(|(name, _)| name.clone()).collect()
    };
    let tcp = TcpTransport::create(&ctx).await?;
    verify_pids(&ctx, &opts, &tcp, cfg, &node_names).await?;

    // Print node states
    for node_name in &node_names {
        let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
        let port = cfg.get_node_port(node_name)?;
        print_query_status(&mut rpc, port, node_name, false).await?;
    }

    Ok(())
}
