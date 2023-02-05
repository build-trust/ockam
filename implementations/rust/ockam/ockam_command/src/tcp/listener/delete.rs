use anyhow::anyhow;
use clap::Args;

use crate::util::extract_address_value;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::util::{node_rpc, Rpc};
use crate::{exitcode, node::NodeOpts, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Listener ID
    pub id: String,
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
    let node = extract_address_value(&cmd.node_opts.api_node)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node)?;
    let req = Request::delete("/node/tcp/listener")
        .body(models::transport::DeleteTransport::new(&cmd.id));
    rpc.request(req).await?;
    if rpc.parse_response::<Vec<u8>>().is_ok() {
        println!("Tcp listener `{}` successfully deleted", cmd.id);
        Ok(())
    } else {
        Err(crate::error::Error::new(
            exitcode::UNAVAILABLE,
            anyhow!(format!("Failed to delete tcp listener `{}`", cmd.id)),
        ))
    }
}
