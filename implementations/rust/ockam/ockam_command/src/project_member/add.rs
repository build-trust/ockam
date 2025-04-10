use crate::node_command::InMemoryNodeCommand;
use crate::project_member::create_member_attributes;
use crate::shared_args::{IdentityOpts, RetryOpts};
use crate::{docs, Command, CommandGlobalOpts, Error};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/add/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/add/after_long_help.txt");

/// This credential attribute allows a member to establish a relay on the Project node.
/// The relay's name should match this attribute's value.
/// If the attribute's value is "*", it implies that any name is acceptable for the relay.
pub const OCKAM_RELAY_ATTRIBUTE: &str = "ockam-relay";

/// Add or update members of a Project, directly as an authorized enroller
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project to which a member should be added
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,

    /// The Identifier of the member to add
    #[arg(value_name = "IDENTIFIER")]
    member: Identifier,

    /// Attributes in `key=value` format to be attached to the member.
    /// You can specify this option multiple times for multiple attributes
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,

    /// Name of the relay that the identity will be allowed to create.
    /// This name is transformed into an attribute, and it's used to prevent collisions with other relays names.
    /// E.g. `--relay foo` is a shorthand for `--attribute ockam-relay=foo`
    #[arg(long = "relay", value_name = "ALLOWED_RELAY_NAME")]
    allowed_relay_name: Option<String>,

    /// Set the enroller role for the member.
    /// When this flag is set, it is transformed into the attribute `ockam-role=enroller`.
    /// This role grants the Identity holding the ticket the ability to enroll other Identities
    /// into the Project, which is a privilege usually reserved for administrators.
    #[arg(long = "enroller")]
    enroller: bool,

    #[command(flatten)]
    retry_opts: RetryOpts,
}

#[derive(Clone)]
struct AddNodeCommand {
    opts: CommandGlobalOpts,
    command: AddCommand,
}

impl AddNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: AddCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for AddNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let authority_node_client = self.authority_client(node.clone()).await?;
        let project = node
            .state()
            .projects()
            .get_project_by_name_or_default(&self.command.project_name)
            .await?;

        let attributes = create_member_attributes(
            &self.command.attributes,
            &self.command.allowed_relay_name,
            self.command.enroller,
        )?;

        authority_node_client
            .add_member(node.ctx(), self.command.member.clone(), attributes.clone())
            .await
            .map_err(Error::Retry)?;

        let output = AddMemberOutput {
            project: project.name().to_string(),
            identifier: self.command.member.clone(),
            attributes,
        };

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(output.to_string())
            .json_obj(&output)?
            .write_line()?;

        Ok(())
    }
}

#[async_trait]
impl Command for AddCommand {
    const NAME: &'static str = "project-member add";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        AddNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

#[derive(Serialize)]
struct AddMemberOutput {
    project: String,
    identifier: Identifier,
    attributes: BTreeMap<String, String>,
}

impl Display for AddMemberOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            fmt_ok!(
                "Identifier {} is now a member of the Project {}",
                color_primary(self.identifier.to_string()),
                color_primary(&self.project)
            )
        )?;
        if !self.attributes.is_empty() {
            let attributes = self
                .attributes
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                f,
                "{}",
                fmt_log!("With attributes: {}", color_primary(attributes))
            )?;
        }
        writeln!(
            f,
            "{}",
            fmt_log!("It can get a credential and access Project resources, like portals of other members")
        )?;
        Ok(())
    }
}
