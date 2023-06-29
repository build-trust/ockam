use clap::Args;
use miette::miette;
use serde::{Deserialize, Serialize};

use ockam::Context;
use ockam::identity::IdentityIdentifier;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::OktaConfig;
use ockam_api::cloud::project::Project;
use ockam_api::config::lookup::ProjectLookup;
use ockam_core::CowStr;

use crate::CommandGlobalOpts;
use crate::error::Error;
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::refresh_projects;
use crate::util::{node_rpc, RpcBuilder};
use crate::util::api::{self, CloudOpts};

#[derive(Clone, Debug, Args)]
pub struct InfoCommand {
    /// Name of the project.
    #[arg(default_value = "default")]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    #[arg(long, default_value = "false")]
    pub as_trust_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectInfo<'a> {
    #[serde(borrow)]
    pub id: CowStr<'a>,
    #[serde(borrow)]
    pub name: CowStr<'a>,
    pub identity: Option<IdentityIdentifier>,
    #[serde(borrow)]
    pub access_route: CowStr<'a>,
    #[serde(borrow)]
    pub authority_access_route: Option<CowStr<'a>>,
    #[serde(borrow)]
    pub authority_identity: Option<CowStr<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub okta_config: Option<OktaConfig>,
}

impl TryFrom<ProjectLookup> for ProjectInfo<'_> {
    type Error = Error;
    fn try_from(p: ProjectLookup) -> Result<Self, Self::Error> {
        Ok(Self {
            id: p.id.into(),
            name: p.name.into(),
            identity: p.identity_id,
            access_route: p
                .node_route
                .map_or(Err(miette!("Project access route is missing")), Ok)?
                .to_string()
                .into(),
            authority_access_route: p.authority.as_ref().map(|a| a.address().to_string().into()),
            authority_identity: p
                .authority
                .as_ref()
                .map(|a| hex::encode(a.identity()).into()),
            okta_config: p.okta.map(|o| o.into()),
        })
    }
}

impl<'a> From<Project> for ProjectInfo<'a> {
    fn from(p: Project) -> Self {
        Self {
            id: p.id.into(),
            name: p.name.into(),
            identity: p.identity,
            access_route: p.access_route.into(),
            authority_access_route: p.authority_access_route.map(|a| a.into()),
            authority_identity: p.authority_identity.map(|a| a.into()),
            okta_config: p.okta_config,
        }
    }
}

impl<'a> From<&ProjectInfo<'a>> for Project {
    fn from(p: &ProjectInfo<'a>) -> Self {
        Project {
            id: p.id.to_string(),
            name: p.name.to_string(),
            identity: p.identity.to_owned(),
            access_route: p.access_route.to_string(),
            authority_access_route: p.authority_access_route.as_ref().map(|a| a.to_string()),
            authority_identity: p.authority_identity.as_ref().map(|a| a.to_string()),
            okta_config: p.okta_config.clone(),
            ..Default::default()
        }
    }
}

impl InfoCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, InfoCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: InfoCommand,
) -> miette::Result<()> {
    let controller_route = &cmd.cloud_opts.route();
    let node_name = start_embedded_node(ctx, &opts, None).await?;

    // Lookup project
    let id = match opts.state.projects.get(&cmd.name) {
        Ok(state) => state.config().id.clone(),
        Err(_) => {
            refresh_projects(ctx, &opts, &node_name, &cmd.cloud_opts.route(), None).await?;
            opts.state.projects.get(&cmd.name)?.config().id.clone()
        }
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::show(&id, controller_route))
        .await?;
    let info: ProjectInfo = rpc.parse_response::<Project>()?.into();

    rpc.print_response(&info)?;

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
