use std::sync::Arc;

use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_api::cloud::AuthorityNodeClient;
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::enroll::okta_oidc_provider::OktaOidcProvider;
use ockam_api::nodes::InMemoryNode;
use ockam_api::NamedTrustContext;

use crate::enroll::OidcServiceExt;
use crate::output::CredentialAndPurposeKeyDisplay;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/enroll/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/enroll/after_long_help.txt");

/// Use an OTC to enroll an identity with a project node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    #[arg(long = "okta", group = "authentication_method")]
    pub okta: bool,

    #[arg(group = "authentication_method", value_name = "ENROLLMENT TICKET PATH | ENROLLMENT TICKET", value_parser = parse_enroll_ticket)]
    pub enroll_ticket: Option<EnrollmentTicket>,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    #[command(flatten)]
    pub trust_opts: TrustContextOpts,

    /// Name of the new trust context to create, defaults to project name
    #[arg(long)]
    pub new_trust_context_name: Option<String>,

    /// Execute enrollment even if the trust context already exists
    #[arg(long)]
    pub force: bool,
}

pub fn parse_enroll_ticket(hex_encoded_data_or_path: &str) -> Result<EnrollmentTicket> {
    let decoded = match std::fs::read_to_string(hex_encoded_data_or_path) {
        Ok(data) => hex::decode(data.trim())
            .into_diagnostic()
            .context("Failed to decode enrollment ticket from file")?,
        Err(_) => hex::decode(hex_encoded_data_or_path)
            .into_diagnostic()
            .context("Failed to decode enrollment ticket from file")?,
    };
    Ok(serde_json::from_slice(&decoded)
        .into_diagnostic()
        .context("Failed to parse enrollment ticket from decoded data")?)
}

impl EnrollCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, EnrollCommand),
) -> miette::Result<()> {
    let identity = opts
        .state
        .get_named_identity_or_default(&cmd.cloud_opts.identity)
        .await?;
    let project = parse_project(&opts, &cmd).await?;
    let trust_context = parse_trust_context(&opts, &cmd, &project).await?;

    // Create secure channel to the project's authority node
    let node = InMemoryNode::start_with_trust_context(
        &ctx,
        &opts.state,
        cmd.trust_opts.project_name,
        Some(trust_context),
    )
    .await?;
    let authority_node: AuthorityNodeClient = node
        .create_authority_client(
            &project.authority_identifier().await.into_diagnostic()?,
            &project.authority_access_route().into_diagnostic()?,
            Some(identity.name()),
        )
        .await?;

    // Enroll
    if let Some(tkn) = cmd.enroll_ticket.as_ref() {
        authority_node
            .present_token(&ctx, &tkn.one_time_code)
            .await?;
    } else if cmd.okta {
        // Get auth0 token
        let okta_config: OktaAuth0 = project
            .okta_config
            .ok_or(miette!("Okta addon not configured"))?
            .into();

        let auth0 = OidcService::new(Arc::new(OktaOidcProvider::new(okta_config)));
        let token = auth0.get_token_interactively(&opts).await?;
        authority_node.enroll_with_oidc_token(&ctx, token).await?;
    };

    // Issue credential
    let credential = authority_node.issue_credential(&ctx).await?;

    opts.terminal
        .clone()
        .stdout()
        .plain(CredentialAndPurposeKeyDisplay(credential))
        .write_line()?;

    Ok(())
}

async fn parse_project(opts: &CommandGlobalOpts, cmd: &EnrollCommand) -> Result<Project> {
    // Retrieve project info from the enrollment ticket or project.json in the case of okta auth
    let project = if let Some(ticket) = &cmd.enroll_ticket {
        let project = ticket
            .project
            .as_ref()
            .expect("Enrollment ticket is invalid. Ticket does not contain a project.")
            .clone();
        opts.state.store_project(project.clone()).await?;
        project
    } else {
        // OKTA AUTHENTICATION FLOW | PREVIOUSLY ENROLLED FLOW
        // currently okta auth does not use an enrollment token
        // however, it could be worked to use one in the future
        //
        // REQUIRES Project passed or default project
        opts.state
            .get_project_by_name_or_default(&cmd.trust_opts.project_name)
            .await
            .context("A default project or project parameter is required.")?
    };
    Ok(project)
}

async fn parse_trust_context(
    opts: &CommandGlobalOpts,
    cmd: &EnrollCommand,
    project: &Project,
) -> Result<NamedTrustContext> {
    let trust_context_name = if let Some(trust_context_name) = &cmd.new_trust_context_name {
        trust_context_name
    } else {
        &project.name
    };

    if !cmd.force {
        if let Ok(trust_context) = opts.state.get_trust_context(trust_context_name).await {
            if trust_context.trust_context_id() != project.id {
                return Err(miette!(
                    "A trust context with the name {} already exists and is associated with a different project. Please choose a different name.",
                    trust_context_name
                ))?;
            }
        }
    }

    let trust_context = opts
        .state
        .create_trust_context(
            Some(trust_context_name.clone()),
            Some(project.id()),
            None,
            project.authority_identity().await.ok(),
            project.authority_access_route().ok(),
        )
        .await?;
    Ok(trust_context)
}
