use std::collections::HashMap;
use std::time::Duration;

use clap::Args;
use miette::{miette, IntoDiagnostic};

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::enrollment_tokens::{Members, TokenIssuer};
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cli_state::CliState;
use ockam_api::cloud::project::Project;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::{proto, MultiAddr, Protocol};

use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::duration::duration_parser;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// Add members to a project as an authorised enroller.
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
    trust_opts: TrustContextOpts,

    #[arg(long, short, conflicts_with = "expires_in")]
    member: Option<Identifier>,

    #[arg(long, short, default_value = "/project/default")]
    to: MultiAddr,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,

    #[arg(long = "expires-in", value_name = "DURATION", conflicts_with = "member", value_parser = duration_parser)]
    expires_in: Option<Duration>,

    #[arg(
        long = "usage-count",
        value_name = "USAGE_COUNT",
        conflicts_with = "member"
    )]
    usage_count: Option<u64>,

    /// The name of the relay that the identity using the ticket will be allowed to create
    #[arg(long = "relay", value_name = "RELAY_NAME")]
    allowed_relay_name: Option<String>,
}

impl TicketCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    fn attributes(&self) -> Result<HashMap<&str, &str>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            let value = parts.next().ok_or(miette!("value expected)"))?;
            attributes.insert(key, value);
        }
        if let Some(relay_name) = &self.allowed_relay_name {
            attributes.insert("ockam-relay", relay_name);
        }
        Ok(attributes)
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, TicketCommand),
) -> miette::Result<()> {
    let trust_context = opts
        .state
        .retrieve_trust_context(
            &cmd.trust_opts.trust_context,
            &cmd.trust_opts.project_name,
            &None,
            &None,
        )
        .await?;
    let node = InMemoryNode::start_with_trust_context(
        &ctx,
        &opts.state,
        cmd.trust_opts.project_name.clone(),
        trust_context,
    )
    .await?;

    let mut project: Option<Project> = None;

    let authority_node = if let Some(name) = cmd.trust_opts.trust_context.as_ref() {
        let authority = if let Some(authority) = opts
            .state
            .get_trust_context(name)
            .await?
            .authority()
            .await
            .into_diagnostic()?
        {
            authority
        } else {
            return Err(miette!(
                "Trust context must be configured with a credential issuer"
            ));
        };

        let identity = opts
            .state
            .get_identity_name_or_default(&cmd.cloud_opts.identity)
            .await?;

        node.create_authority_client(&authority.identifier(), &authority.route(), Some(identity))
            .await?
    } else if let Some(p) = get_project(&opts.state, &cmd.to).await? {
        let identity = opts
            .state
            .get_identity_name_or_default(&cmd.cloud_opts.identity)
            .await?;
        project = Some(p.clone());
        node.create_authority_client(
            &p.authority_identifier().await.into_diagnostic()?,
            &p.authority_access_route().into_diagnostic()?,
            Some(identity),
        )
        .await?
    } else {
        return Err(miette!("Cannot create a ticket. Please specify a route to your project or to an authority node"));
    };
    // If an identity identifier is given add it as a member, otherwise
    // request an enrollment token that a future member can use to get a
    // credential.
    if let Some(id) = &cmd.member {
        authority_node
            .add_member(&ctx, id.clone(), cmd.attributes()?)
            .await?
    } else {
        let token = authority_node
            .create_token(&ctx, cmd.attributes()?, cmd.expires_in, cmd.usage_count)
            .await?;

        let ticket = EnrollmentTicket::new(token, project);
        let ticket_serialized = ticket.hex_encoded().into_diagnostic()?;
        opts.terminal
            .clone()
            .stdout()
            .machine(ticket_serialized)
            .write_line()?;
    }

    Ok(())
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
async fn get_project(cli_state: &CliState, input: &MultiAddr) -> Result<Option<Project>> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let project_name = proto.cast::<proto::Project>().expect("project protocol");
            match cli_state.get_project_by_name(&project_name).await.ok() {
                None => Err(miette!("unknown project {}", project_name.to_string()))?,
                Some(project) => {
                    if project.authority_identifier().await.is_err() {
                        Err(miette!(
                            "missing authority in project {}",
                            project_name.to_string()
                        ))?
                    } else {
                        Ok(Some(project))
                    }
                }
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
