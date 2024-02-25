use std::collections::BTreeMap;
use std::time::Duration;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::{
    Members, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::authenticator::enrollment_tokens::TokenIssuer;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cli_state::CliState;
use ockam_api::cloud::project::Project;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::{proto, MultiAddr, Protocol};

use crate::fmt_ok;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Result};
use crate::{
    output::OutputFormat,
    util::api::{CloudOpts, TrustOpts},
};
use crate::{terminal::color_primary, util::duration::duration_parser};
use ockam_api::cloud::project::models::ProjectModel;
use tracing::debug;

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// This attribute in credential allows member to create a relay on the Project node, the name of the relay should be
/// equal to the value of that attribute. If the value is `*` then any name is allowed
pub const OCKAM_RELAY_ATTRIBUTE: &str = "ockam-relay";

/// Add members to a project, as an authorized enroller, directly or via an enrollment ticket
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct TicketCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_opts: TrustOpts,

    /// Bypass ticket creation, add this member directly to the project's authority, with the given attributes
    #[arg(value_name = "IDENTIFIER", long, short, conflicts_with = "expires_in")]
    member: Option<Identifier>,

    /// The project name from this option is used to create the enrollment ticket. This takes precedence over `--project`
    #[arg(
        long,
        short,
        default_value = "/project/default",
        value_name = "ROUTE_TO_PROJECT"
    )]
    to: MultiAddr,

    /// Attributes in `key=value` format to be attached to the member. You can specify this option multiple times for multiple attributes
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,

    // Note: MAX_TOKEN_DURATION holds the default value.
    /// Duration for which the enrollment ticket is valid, if you don't specify this, the default is 10 minutes. Examples: 10000ms, 600s, 600, 10m, 1h, 1d. If you don't specify a length sigil, it is assumed to be seconds
    #[arg(long = "expires-in", value_name = "DURATION", conflicts_with = "member", value_parser = duration_parser)]
    expires_in: Option<Duration>,

    /// Number of times the ticket can be used to enroll, the default is 1
    #[arg(
        long = "usage-count",
        value_name = "USAGE_COUNT",
        conflicts_with = "member"
    )]
    usage_count: Option<u64>,

    /// Name of the relay that the identity using the ticket will be allowed to create. This name is transformed into attributes to prevent collisions when creating relay names. For example: `--relay foo` is shorthand for `--attribute ockam-relay=foo`
    #[arg(long = "relay", value_name = "ENROLLEE_ALLOWED_RELAY_NAME")]
    allowed_relay_name: Option<String>,

    /// Add the enroller role to your ticket. If you specify it, this flag is transformed into the attributes `--attribute ockam-role=enroller`. This role allows the identity using the ticket to enroll other identities into the project, typically something that only admins can do
    #[arg(long = "enroller")]
    enroller: bool,
}

impl TicketCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create project ticket".into()
    }

    fn attributes(&self) -> Result<BTreeMap<String, String>> {
        let mut attributes = BTreeMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            let value = parts.next().ok_or(miette!("value expected)"))?;
            attributes.insert(key.to_string(), value.to_string());
        }
        if let Some(relay_name) = self.allowed_relay_name.clone() {
            attributes.insert(OCKAM_RELAY_ATTRIBUTE.to_string(), relay_name);
        }
        if self.enroller {
            attributes.insert(
                OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
                OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
            );
        }
        Ok(attributes)
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.global_args.output_format == OutputFormat::Json {
            return Err(miette::miette!(
            "This command only outputs a hex encoded string for 'ockam project enroll' to use. \
            Please try running it again without '--output json'."
        ));
        }

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            self.trust_opts.project_name.clone(),
        )
        .await?;

        let project_model: Option<ProjectModel>;

        let authority_node_client = if let Some(p) = get_project(&opts.state, &self.to).await? {
            let identity = opts
                .state
                .get_identity_name_or_default(&self.cloud_opts.identity)
                .await?;
            project_model = Some(p.model().clone());
            node.create_authority_client(
                &p.authority_identifier().into_diagnostic()?,
                p.authority_multiaddr().into_diagnostic()?,
                Some(identity),
            )
            .await?
        } else {
            return Err(miette!("Cannot create a ticket. Please specify a route to your project or to an authority node"));
        };

        let attributes = self.attributes()?;
        debug!(attributes = ?attributes, "Attributes passed");

        // If an identity identifier is given add it as a member, otherwise
        // request an enrollment token that a future member can use to get a
        // credential.
        if let Some(id) = &self.member {
            authority_node_client
                .add_member(ctx, id.clone(), attributes)
                .await?
        } else {
            let token = authority_node_client
                .create_token(ctx, attributes, self.expires_in, self.usage_count)
                .await?;

            let ticket = EnrollmentTicket::new(token, project_model);
            let ticket_serialized = ticket.hex_encoded().into_diagnostic()?;

            opts.terminal.write_line(&fmt_ok!(
                "{}: {}",
                "Created enrollment ticket. You can use it to enroll another machine using",
                color_primary("ockam project enroll")
            ))?;

            opts.terminal
                .clone()
                .stdout()
                .machine(ticket_serialized)
                .write_line()?;
        }

        Ok(())
    }
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
async fn get_project(cli_state: &CliState, input: &MultiAddr) -> Result<Option<Project>> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let project_name = proto.cast::<proto::Project>().expect("project protocol");
            match cli_state.projects().get_project_by_name(&project_name).await.ok() {
                None => Err(miette!("Unknown project '{}'. Run 'ockam project list' to get a list of available projects.", project_name.to_string()))?,
                Some(project) => {
                    Ok(Some(project))
                }
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
