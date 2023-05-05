use clap::Args;
use colorful::Colorful;

use crate::util::parse_node_name;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::util::{node_rpc, Rpc};
use crate::{node::NodeOpts, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Listener ID
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let node = parse_node_name(&cmd.node_opts.api_node)?;
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
