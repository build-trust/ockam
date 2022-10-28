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
use slug::slugify;

/// Create vaults
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: Option<NodeOpts>,

    /// Path to the Vault storage file
    #[arg(short, long, requires = "node")]
    pub path: Option<String>,

    #[arg(long = "name", conflicts_with = "node")]
    vault_name: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(ctx: Context, (options, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    match (cmd.node_opts, cmd.vault_name) {
        (Some(node_opts), None) => {
            let node_name = node_opts.api_node.clone();
            let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
            let request = Request::post("/node/vault").body(CreateVaultRequest::new(cmd.path));
            rpc.request(request).await?;
            rpc.is_ok()?;
            println!("Vault created for the Node {}!", node_name);
        }
        (None, Some(vault_name)) => {
            let dirs = cli::OckamConfig::directories();
            let dir = dirs
                .config_dir()
                .to_path_buf()
                .join("vaults")
                .join(slugify(&format!("vault-{}", vault_name)));
            if dir.as_path().exists() {
                return Err(crate::error::Error::new(
                    CANTCREAT,
                    anyhow!("Vault with name {} already exists!", vault_name),
                ));
            }
            tokio::fs::create_dir_all(dir.as_path()).await?;
            let file = dir.join("vault.json");
            let _ = FileStorage::create(file.clone()).await?;
            println!("Vault created with name: {}!", vault_name);
        }
        _ => unreachable!(),
    }
    Ok(())
}
