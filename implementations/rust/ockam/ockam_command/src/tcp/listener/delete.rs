use clap::Args;
use colorful::Colorful;

use crate::util::parse_node_name;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::{node_rpc, Rpc};
use crate::{node::NodeOpts, CommandGlobalOpts};

/// Delete a TCP listener
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Listener ID
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.api_node);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.api_node);
    let node = parse_node_name(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node)?;
    let req = Request::delete("/node/tcp/listener")
        .body(models::transport::DeleteTransport::new(cmd.address.clone()));
    rpc.request(req).await?;
    rpc.is_ok()?;

    opts.terminal
        .stdout()
        .plain(format!(
            "{} TCP listener with address '{}' has been deleted.",
            "✔︎".light_green(),
            &cmd.address
        ))
        .machine(&cmd.address)
        .json(serde_json::json!({ "tcp-listener": { "address": &cmd.address } }))
        .write_line()?;

    Ok(())
}
