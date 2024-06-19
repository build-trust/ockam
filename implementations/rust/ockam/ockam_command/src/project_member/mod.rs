use std::collections::BTreeMap;

use clap::Args;
use clap::Subcommand;
use miette::miette;
use serde::Serialize;
use std::fmt::Write;

use add::{AddCommand, OCKAM_RELAY_ATTRIBUTE};
use delete::DeleteCommand;
use list::ListCommand;
use list_ids::ListIdsCommand;
use ockam::identity::{AttributesEntry, Identifier};
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::AuthorityNodeClient;
use ockam_api::colors::{color_primary, color_warn};
use ockam_api::nodes::{InMemoryNode, NodeManager};
use ockam_api::output::Output;
use ockam_api::terminal::fmt;
use ockam_api::CliState;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use ockam_node::Context;

use crate::project_member::show::ShowCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts};

mod add;
mod delete;
mod list;
mod list_ids;
mod show;

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
            ProjectMemberSubcommand::Show(c) => c.run(opts),
            ProjectMemberSubcommand::Delete(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ProjectMemberSubcommand::List(c) => c.name(),
            ProjectMemberSubcommand::ListIds(c) => c.name(),
            ProjectMemberSubcommand::Add(c) => c.name(),
            ProjectMemberSubcommand::Show(c) => c.name(),
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
    Show(ShowCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
}

pub(super) async fn authority_client(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    identity_opts: &IdentityOpts,
    project_route: &Option<MultiAddr>,
) -> crate::Result<(AuthorityNodeClient, String)> {
    let project = get_project(&opts.state, project_route).await?;
    let node =
        InMemoryNode::start_with_project_name(ctx, &opts.state, Some(project.name().to_string()))
            .await?;
    Ok((
        create_authority_client(&node, &opts.state, identity_opts, &project).await?,
        project.name().to_string(),
    ))
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
        .get_identity_name_or_default(&identity_opts.identity_name)
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
        let key = parts
            .next()
            .ok_or(miette!("key expected in attribute {attr}"))?;
        let value = parts
            .next()
            .ok_or(miette!("value expected in attribute {attr}"))?;
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

#[derive(Serialize)]
struct MemberOutput {
    identifier: Identifier,
    attributes: AttributesEntry,
}

impl MemberOutput {
    fn new(identifier: Identifier, attributes: AttributesEntry) -> Self {
        Self {
            identifier,
            attributes,
        }
    }

    fn to_string(&self, padding: &str) -> ockam_api::Result<String> {
        let mut f = String::new();
        writeln!(
            f,
            "{}{}",
            padding,
            color_primary(self.identifier.to_string())
        )?;

        if self.attributes.attrs().is_empty() {
            writeln!(f, "{}Has no attributes", padding)?;
        } else {
            let attributes = self.attributes.deserialized_key_value_attrs();
            writeln!(
                f,
                "{}With attributes: {}",
                padding,
                color_primary(attributes.join(", "))
            )?;
            writeln!(
                f,
                "{}{}Added at: {}",
                padding,
                fmt::INDENTATION,
                color_warn(self.attributes.added_at().to_string())
            )?;
            if let Some(expires_at) = self.attributes.expires_at() {
                writeln!(
                    f,
                    "{}{}Expires at: {}",
                    padding,
                    fmt::INDENTATION,
                    color_warn(expires_at.to_string())
                )?;
            }
            if let Some(attested_by) = &self.attributes.attested_by() {
                writeln!(
                    f,
                    "{}{}Attested by: {}",
                    padding,
                    fmt::INDENTATION,
                    color_primary(attested_by.to_string())
                )?;
            }
        }
        Ok(f)
    }
}

impl Output for MemberOutput {
    fn item(&self) -> ockam_api::Result<String> {
        self.to_string(fmt::PADDING)
    }

    fn as_list_item(&self) -> ockam_api::Result<String> {
        self.to_string("")
    }
}
