use crate::node::NodeOpts;
use crate::state::VaultConfig;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::{help, state};
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::identity::CreateIdentityResponse;
use ockam_core::api::Request;
use ockam_identity::Identity;
use rand::prelude::random;

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: Option<NodeOpts>,

    #[arg(conflicts_with = "node", hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    name: String,

    /// Vault name to store the identity key
    #[arg(long)]
    vault: Option<String>,
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
    if let Some(node_opts) = cmd.node_opts {
        let node_name = node_opts.api_node.clone();
        let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
        let request = Request::post("/node/identity");
        rpc.request(request).await?;
        let res = rpc.parse_response::<CreateIdentityResponse>()?;
        println!("Identity created: {}", res.identity_id);
    } else {
        let vault_config = if let Some(vault_name) = cmd.vault {
            options.state.vaults.get(&vault_name)?.config
        } else if options.state.vaults.default().is_err() {
            let vault_name = hex::encode(random::<[u8; 4]>());
            let config = options
                .state
                .vaults
                .create(&vault_name, VaultConfig::fs_default(&vault_name)?)
                .await?
                .config;
            println!("Default vault created: {}", &vault_name);
            config
        } else {
            options.state.vaults.default()?.config
        };
        let vault = vault_config.get().await?;
        let identity = Identity::create(&ctx, &vault).await?;
        let identity_config = state::IdentityConfig::new(&identity).await;
        options
            .state
            .identities
            .create(&cmd.name, identity_config)?;
        println!("Identity created: {}", identity.identifier());
    }
    Ok(())
}
