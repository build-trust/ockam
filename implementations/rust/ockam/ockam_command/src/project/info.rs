use anyhow::Context as _;
use clap::Args;

use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::cloud::project::Project;
use ockam_core::CowStr;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Args)]
pub struct InfoCommand {
    /// Name of the project.
    #[clap(long)]
    pub name: String,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectInfo<'a> {
    #[serde(borrow)]
    pub id: CowStr<'a>,
    pub identity: Option<IdentityIdentifier>,
    #[serde(borrow)]
    pub authority_access_route: Option<CowStr<'a>>,
    #[serde(borrow)]
    pub authority_identity: Option<CowStr<'a>>,
}

impl<'a> From<Project<'a>> for ProjectInfo<'a> {
    fn from(p: Project<'a>) -> Self {
        Self {
            id: p.id,
            identity: p.identity,
            authority_access_route: p.authority_access_route,
            authority_identity: p.authority_identity,
        }
    }
}

impl InfoCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, InfoCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: InfoCommand,
) -> crate::Result<()> {
    let controller_route = cmd.cloud_opts.route();
    let node_name = start_embedded_node(ctx, &opts.config).await?;

    // Lookup project
    let id = match config::get_project(&opts.config, &cmd.name) {
        Some(id) => id,
        None => {
            config::refresh_projects(ctx, &opts, &node_name, cmd.cloud_opts.route(), None).await?;
            config::get_project(&opts.config, &cmd.name)
                .context(format!("Project '{}' does not exist", cmd.name))?
        }
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::show(&id, controller_route))
        .await?;
    let info: ProjectInfo = rpc.parse_response::<Project>()?.into();
    rpc.print_response(&info)?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
