use crate::node::NodeOpts;
use crate::util::exitcode::CANTCREAT;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::{ArgGroup, Args};
use ockam::Context;
use ockam_api::config::cli;
use ockam_api::nodes::models::identity::CreateIdentityResponse;
use ockam_core::api::Request;
use ockam_vault::storage::FileStorage;
use slug::slugify;

#[derive(Clone, Debug, Args)]
#[command(group(
   ArgGroup::new("i")
    .args(["node", "identity_name"]),
), arg_required_else_help = true)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: Option<NodeOpts>,

    #[arg(long = "identity", conflicts_with = "node", requires = "vault_name")]
    identity_name: Option<String>,

    #[arg(long = "vault", conflicts_with = "node", requires = "identity_name")]
    vault_name: Option<String>,
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
    match (cmd.node_opts, cmd.identity_name, cmd.vault_name) {
        (Some(node_opts), None, None) => {
            let mut rpc = Rpc::background(&ctx, &options, &node_opts.api_node)?;
            let request = Request::post("/node/identity");
            rpc.request(request).await?;
            let res = rpc.parse_response::<CreateIdentityResponse>()?;
            println!("Identity {} created!", res.identity_id);
            Ok(())
        }
        (None, Some(identity_name), Some(vault_name)) => {
            let dirs = cli::OckamConfig::directories();
            let dir = dirs
                .config_dir()
                .to_path_buf()
                .join("vaults")
                .join(slugify(&format!("vault-{}", vault_name)));

            if !dir.as_path().exists() {
                return Err(crate::error::Error::new(
                    CANTCREAT,
                    anyhow!("Vault with name {} doesn't exists!", vault_name),
                ));
            }

            let identity_path = format!("{}.json", slugify(&format!("identity-{}", identity_name)));

            if dir.join(&identity_path).as_path().exists() {
                return Err(crate::error::Error::new(
                    CANTCREAT,
                    anyhow!("Identity with name {} already exists!", identity_name),
                ));
            }

            let file = dir.join(&identity_path);
            let _ = FileStorage::create(file.clone()).await?;

            println!("Identity {} created in vault {}", identity_name, vault_name);
            Ok(())
        }
        _ => unreachable!(),
    }
}
