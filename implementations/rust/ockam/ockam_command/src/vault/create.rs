use crate::node::NodeOpts;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::vault::CreateVaultRequest;
use ockam_core::api::Request;

/// Create vaults
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Path to the Vault storage file
    #[arg(short, long)]
    pub path: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let mut rpc = Rpc::background(&ctx, &options, &cmd.node_opts.api_node)?;
    let request = Request::post("/node/vault").body(CreateVaultRequest::new(cmd.path));

    rpc.request(request).await?;
    rpc.parse_response()?;

    println!("Vault created!");
    Ok(())
}
