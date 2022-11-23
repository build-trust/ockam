use crate::node::NodeOpts;
use crate::util::{node_rpc, Rpc};
use crate::Result;
use crate::{state, CommandGlobalOpts};
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::vault::CreateVaultRequest;
use ockam_core::api::Request;
use rand::prelude::random;

/// Create vaults
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: Option<NodeOpts>,

    #[arg(conflicts_with = "node", hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    name: String,

    /// Path to the Vault storage file
    #[arg(short, long)]
    path: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(ctx: Context, (options, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    if let Some(node_opts) = cmd.node_opts {
        let node_name = node_opts.api_node.clone();
        let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
        let request = Request::post("/node/vault").body(CreateVaultRequest::new(cmd.path));
        rpc.request(request).await?;
        rpc.is_ok()?;
        println!("Vault created for the Node {}!", node_name);
    } else {
        let path = state::VaultConfig::fs_path(&cmd.name, cmd.path)?;
        let config = state::VaultConfig::fs(path).await?;
        options.state.vaults.create(&cmd.name, config)?;
        println!("Vault created with name: {}!", &cmd.name);
    }
    Ok(())
}
