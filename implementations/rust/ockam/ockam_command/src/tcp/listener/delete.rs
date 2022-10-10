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

    /// Force this operation: delete the API transport if requested
    #[arg(long)]
    pub force: bool,
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
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force));
    rpc.request(req).await?;
    if rpc.parse_response::<Vec<u8>>().is_ok() {
        println!("Tcp listener `{}` successfully deleted", cmd.id);
        Ok(())
    } else {
        let mut msg = "Failed to delete tcp listener".to_string();
        if !cmd.force {
            msg.push_str("\nYou may have to provide --force to delete the API transport");
        }
        Err(crate::error::Error::new(
            exitcode::UNAVAILABLE,
            anyhow!(msg),
        ))
    }
}
