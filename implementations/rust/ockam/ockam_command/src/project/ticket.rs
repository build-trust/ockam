use crate::util::duration::duration_parser;
use clap::Args;
use ockam_api::config::cli::TrustContextConfig;
use ockam_api::identity::EnrollmentTicket;
use std::collections::HashMap;
use std::time::Duration;

use miette::{miette, IntoDiagnostic};
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::enrollment_tokens::{Members, TokenIssuer};
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::config::lookup::{ProjectAuthority, ProjectLookup};
use ockam_multiaddr::{proto, MultiAddr, Protocol};

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::LocalNode;
use crate::util::api::{CloudOpts, TrustContextOpts};
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

    #[arg(long = "expires-in", value_name = "DURATION", conflicts_with = "member", value_parser=duration_parser)]
    expires_in: Option<Duration>,
}

impl TicketCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        initialize_identity_if_default(&options, &self.cloud_opts.identity);
        node_rpc(
            |ctx, (opts, cmd)| Runner::new(ctx, opts, cmd).run(),
            (options, self),
        );
    }

    fn attributes(&self) -> Result<HashMap<&str, &str>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            let value = parts.next().ok_or(miette!("value expected)"))?;
            attributes.insert(key, value);
        }
        Ok(attributes)
    }
}

struct Runner {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: TicketCommand,
}

impl Runner {
    fn new(ctx: Context, opts: CommandGlobalOpts, cmd: TicketCommand) -> Self {
        Self { ctx, opts, cmd }
    }

    async fn run(self) -> miette::Result<()> {
        let node = LocalNode::make(&self.ctx, &self.opts, Some(&self.cmd.trust_opts)).await?;

        let mut project: Option<ProjectLookup> = None;
        let mut trust_context: Option<TrustContextConfig> = None;

        let authority_node = if let Some(tc) = self.cmd.trust_opts.trust_context.as_ref() {
            let tc = &self.opts.state.trust_contexts.read_config_from_path(tc)?;
            trust_context = Some(tc.clone());
            let cred_retr = tc
                .authority()
                .into_diagnostic()?
                .own_credential()
                .into_diagnostic()?;
            let addr = match cred_retr {
                ockam_api::config::cli::CredentialRetrieverConfig::FromCredentialIssuer(c) => {
                    &c.multiaddr
                }
                _ => {
                    return Err(miette!(
                        "Trust context must be configured with a credential issuer"
                    ));
                }
            };
            let identity = get_identity_name(&self.opts.state, &self.cmd.cloud_opts.identity);
            let authority_identifier = tc
                .authority()
                .into_diagnostic()?
                .identity()
                .await
                .into_diagnostic()?
                .identifier().clone();

            node.make_authority_node_client(authority_identifier, addr.clone(), Some(identity))
                .await?
        } else if let (Some(p), Some(a)) = get_project(&self.opts.state, &self.cmd.to).await? {
            let identity = get_identity_name(&self.opts.state, &self.cmd.cloud_opts.identity);
            project = Some(p);
            node.make_authority_node_client(
                a.identity_id().clone(),
                a.address().clone(),
                Some(identity),
            )
            .await?
        } else {
            return Err(miette!("Cannot create a ticket. Please specify a route to your project or to an authority node"));
        };
        // If an identity identifier is given add it as a member, otherwise
        // request an enrollment token that a future member can use to get a
        // credential.
        if let Some(id) = &self.cmd.member {
            authority_node
                .add_member(&self.ctx, id.clone(), self.cmd.attributes()?)
                .await?
        } else {
            let token = authority_node
                .create_token(&self.ctx, self.cmd.attributes()?, self.cmd.expires_in)
                .await?;

            let ticket = EnrollmentTicket::new(token, project, trust_context);
            let ticket_serialized = ticket.hex_encoded().into_diagnostic()?;
            self.opts
                .terminal
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
async fn get_project(
    cli_state: &CliState,
    input: &MultiAddr,
) -> Result<(Option<ProjectLookup>, Option<ProjectAuthority>)> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let proj = proto.cast::<proto::Project>().expect("project protocol");
            return if let Ok(p) = cli_state.projects.get(proj.to_string()) {
                let c = p.config();
                let a =
                    ProjectAuthority::from_raw(&c.authority_access_route, &c.authority_identity)
                        .await?;
                if a.is_some() {
                    let p = ProjectLookup::from_project(c).await?;
                    Ok((Some(p), a))
                } else {
                    Err(miette!("missing authority in project {:?}", &*proj).into())
                }
            } else {
                Err(miette!("unknown project {}", &*proj).into())
            };
        }
    }
    Ok((None, None))
}
