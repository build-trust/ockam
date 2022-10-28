use crate::node::NodeOpts;
use crate::util::exitcode::CANTCREAT;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use anyhow::anyhow;
use clap::Args;
use ockam::Context;
use ockam_api::config::cli;
use ockam_api::nodes::models::vault::CreateVaultRequest;
use ockam_core::api::Request;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use slug::slugify;
use std::sync::Arc;

/// Create vaults
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: Option<NodeOpts>,

    #[arg(long, conflicts_with = "node")]
    vault_name: Option<String>,

    /// Path to the Vault storage file
    #[arg(short, long)]
    pub path: Option<String>,
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
    } else if let Some(vault_name) = cmd.vault_name {
        create_new_vault(vault_name.clone()).await?;
    }

    Ok(())
}

async fn create_new_vault(vault_name: String) -> Result<()> {
    let directories = cli::OckamConfig::directories();
    let dir = directories
        .config_dir()
        .to_path_buf()
        .join("vaults")
        .join(slugify(&format!("vault-{}", vault_name)));

    if dir.as_path().exists() {
        return Err(crate::error::Error::new(
            CANTCREAT,
            anyhow!("Vault with name {} already exists!", vault_name),
        ));
    } else {
        tokio::fs::create_dir_all(dir.as_path()).await?;
        let file = dir.join("vault.json");
        let storage = FileStorage::create(file.clone()).await?;
        let _ = Vault::new(Some(Arc::new(storage)));

        println!("Vault created with name: {}!", vault_name);
    }

    Ok(())
}
