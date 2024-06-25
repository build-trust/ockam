use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tracing::debug;

use ockam::Context;
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::authenticator::enrollment_tokens::TokenIssuer;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok};
use ockam_multiaddr::MultiAddr;

use crate::shared_args::{IdentityOpts, RetryOpts, TrustOpts};
use crate::util::parsers::duration_parser;
use crate::{docs, Command, CommandGlobalOpts, Error, Result};

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// This attribute in credential allows member to create a relay on the Project node, the name of the relay should be
/// equal to the value of that attribute. If the value is `*` then any name is allowed
pub const OCKAM_RELAY_ATTRIBUTE: &str = "ockam-relay";

/// Add members to a Project, as an authorized enroller, directly, or via an enrollment ticket
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct TicketCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    trust_opts: TrustOpts,

    /// The Project name from this option is used to create the enrollment ticket. This takes precedence over `--project`
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,

    /// Attributes in `key=value` format to be attached to the member. You can specify this option multiple times for multiple attributes
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,

    // Note: MAX_TOKEN_DURATION holds the default value.
    /// Duration for which the enrollment ticket is valid, if you don't specify this, the default is 10 minutes. Examples: 10000ms, 600s, 600, 10m, 1h, 1d. If you don't specify a length sigil, it is assumed to be seconds
    #[arg(long = "expires-in", value_name = "DURATION", value_parser = duration_parser)]
    expires_in: Option<Duration>,

    /// Number of times the ticket can be used to enroll, the default is 1
    #[arg(long = "usage-count", value_name = "USAGE_COUNT")]
    usage_count: Option<u64>,

    /// Name of the relay that the identity using the ticket will be allowed to create. This name is transformed into attributes to prevent collisions when creating relay names. For example: `--relay foo` is shorthand for `--attribute ockam-relay=foo`
    #[arg(long = "relay", value_name = "ENROLLEE_ALLOWED_RELAY_NAME")]
    allowed_relay_name: Option<String>,

    /// Add the enroller role to your ticket. If you specify it, this flag is transformed into the attributes `--attribute ockam-role=enroller`. This role allows the Identity using the ticket to enroll other Identities into the Project, typically something that only admins can do
    #[arg(long = "enroller")]
    enroller: bool,

    #[command(flatten)]
    retry_opts: RetryOpts,
}

#[async_trait]
impl Command for TicketCommand {
    const NAME: &'static str = "project ticket";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        if opts.global_args.output_format()?.is_json() {
            return Err(miette::miette!(
                "This command only outputs a hex encoded string for 'ockam project enroll' to use. \
                Please try running it again without '--output json'."
            )
            .into());
        }

        let project = crate::project_member::get_project(&opts.state, &self.to).await?;

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;

        let identity = opts
            .state
            .get_identity_name_or_default(&self.identity_opts.identity_name)
            .await?;

        let authority_node_client = node
            .create_authority_client(&project, Some(identity))
            .await?;

        let attributes = self.attributes()?;
        debug!(attributes = ?attributes, "Attributes passed");

        // Request an enrollment token that a future member can use to get a
        // credential.
        let token = authority_node_client
            .create_token(ctx, attributes, self.expires_in, self.usage_count)
            .await
            .map_err(Error::Retry)?;

        let ticket = EnrollmentTicket::new(token, Some(project.model().clone()));
        let ticket_serialized = ticket.hex_encoded().into_diagnostic()?;

        opts.terminal
            .write_line(fmt_ok!("Created enrollment ticket\n"))?;
        opts.terminal.write_line(fmt_log!(
            "You can use it to enroll another machine using: {}",
            color_primary("ockam project enroll")
        ))?;

        opts.terminal
            .stdout()
            .machine(ticket_serialized)
            .write_line()?;

        Ok(())
    }
}

impl TicketCommand {
    fn attributes(&self) -> Result<BTreeMap<String, String>> {
        let mut attributes = BTreeMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            // If no value is provided we assume that the attribute is a boolean attribute set to "true"
            let value = parts.next().unwrap_or("true");
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
}
