use crate::util::api::IdentityOpts;
use crate::{docs, CommandGlobalOpts};
use add::{AddCommand, OCKAM_RELAY_ATTRIBUTE};
use clap::Args;
use clap::Subcommand;
use delete::DeleteCommand;
use list::ListCommand;
use list_ids::ListIdsCommand;
use miette::miette;
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::AuthorityNodeClient;
use ockam_api::nodes::NodeManager;
use ockam_api::CliState;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use std::collections::BTreeMap;

mod add;
mod delete;
mod list;
mod list_ids;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Authority nodes
#[derive(Clone, Debug, Args)]
#[command(
    hide = docs::hide(),
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct ProjectMemberCommand {
    #[command(subcommand)]
    pub(crate) subcommand: ProjectMemberSubcommand,
}

impl ProjectMemberCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ProjectMemberSubcommand::List(c) => c.run(opts),
            ProjectMemberSubcommand::ListIds(c) => c.run(opts),
            ProjectMemberSubcommand::Add(c) => c.run(opts),
            ProjectMemberSubcommand::Delete(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ProjectMemberSubcommand::List(c) => c.name(),
            ProjectMemberSubcommand::ListIds(c) => c.name(),
            ProjectMemberSubcommand::Add(c) => c.name(),
            ProjectMemberSubcommand::Delete(c) => c.name(),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectMemberSubcommand {
    #[command(display_order = 800)]
    ListIds(ListIdsCommand),
    #[command(display_order = 800)]
    List(ListCommand),
    #[command(display_order = 800)]
    Add(AddCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
pub(super) async fn get_project(
    cli_state: &CliState,
    input: &Option<MultiAddr>,
) -> crate::Result<Project> {
    let project_name = match input {
        Some(input) => match input.first() {
            Some(proto) if proto.code() == proto::Project::CODE => Some(
                proto
                    .cast::<proto::Project>()
                    .expect("project protocol")
                    .to_string(),
            ),
            _ => return Err(miette!("Invalid project address '{}'.", input.to_string()))?,
        },
        None => None,
    };

    match cli_state
        .projects()
        .get_project_by_name_or_default(&project_name)
        .await
        .ok()
    {
        None => Err(miette!(
            "Project not found. Run 'ockam project list' to get a list of available projects.",
        ))?,
        Some(project) => Ok(project),
    }
}

pub(super) async fn create_authority_client(
    node: &NodeManager,
    cli_state: &CliState,
    identity_opts: &IdentityOpts,
    project: &Project,
) -> crate::Result<AuthorityNodeClient> {
    let identity = cli_state
        .get_identity_name_or_default(&identity_opts.identity)
        .await?;

    Ok(node
        .create_authority_client(project, Some(identity))
        .await?)
}

pub(crate) fn create_member_attributes(
    attrs: &[String],
    allowed_relay_name: &Option<String>,
    enroller: bool,
) -> crate::Result<BTreeMap<String, String>> {
    let mut attributes = BTreeMap::new();
    for attr in attrs {
        let mut parts = attr.splitn(2, '=');
        let key = parts.next().ok_or(miette!("key expected"))?;
        let value = parts.next().ok_or(miette!("value expected)"))?;
        attributes.insert(key.to_string(), value.to_string());
    }
    if let Some(relay_name) = allowed_relay_name {
        attributes.insert(OCKAM_RELAY_ATTRIBUTE.to_string(), relay_name.clone());
    }
    if enroller {
        attributes.insert(
            OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
            OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
        );
    }
    Ok(attributes)
}
