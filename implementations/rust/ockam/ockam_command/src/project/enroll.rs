use std::sync::Arc;

use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cli_state::{ProjectConfigCompact, StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_api::cloud::AuthorityNode;
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::enroll::okta_oidc_provider::OktaOidcProvider;
use ockam_api::identity::EnrollmentTicket;

use crate::enroll::OidcServiceExt;
use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::InMemoryNode;
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
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, EnrollCommand),
) -> miette::Result<()> {
    let result = project_enroll(&ctx, &opts, cmd).await;
    result.map(|_| ())
}

pub async fn project_enroll(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: EnrollCommand,
) -> miette::Result<String> {
    let project = retrieve_project(opts, &cmd).await?;
    let project_authority = project
        .authority()
        .await
        .into_diagnostic()?
        .ok_or_else(|| miette!("Authority details not configured"))?;
    let identity_name = get_identity_name(&opts.state, &cmd.cloud_opts.identity);

    // Create secure channel to the project's authority node
    let node = InMemoryNode::create(ctx, &opts.state, Some(&cmd.trust_opts)).await?;
    let authority_node: AuthorityNode = node
        .make_authority_node_client(
            project_authority.identity_id(),
            project_authority.address(),
            Some(identity_name),
        )
        .await?;

    if let Some(tkn) = cmd.enroll_ticket.as_ref() {
        authority_node
            .present_token(ctx, &tkn.one_time_code)
            .await?;
    } else if cmd.okta {
        // Get auth0 token
        let okta_config: OktaAuth0 = project
            .okta_config
            .ok_or(miette!("Okta addon not configured"))?
            .into();

        let auth0 = OidcService::new(Arc::new(OktaOidcProvider::new(okta_config)));
        let token = auth0.get_token_interactively(opts).await?;
        authority_node.enroll_with_oidc_token(ctx, token).await?;
    };

    let credential = authority_node.issue_credential(ctx).await?;
    opts.terminal
        .clone()
        .stdout()
        .plain(CredentialAndPurposeKeyDisplay(credential))
        .write_line()?;

    Ok(project.name)
}

async fn retrieve_project(opts: &CommandGlobalOpts, cmd: &EnrollCommand) -> Result<Project> {
    let project_as_string: String;

    // Retrieve project info from the enrollment ticket or project.json in the case of okta auth
    let proj: ProjectConfigCompact = if let Some(ticket) = &cmd.enroll_ticket {
        ticket
            .project
            .as_ref()
            .expect("Enrollment ticket is invalid. Ticket does not contain a project.")
            .clone()
            .try_into()?
    } else {
        // OKTA AUTHENTICATION FLOW | PREVIOUSLY ENROLLED FLOW
        // currently okta auth does not use an enrollment token
        // however, it could be worked to use one in the future
        //
        // REQUIRES Project passed or default project
        let path = match cmd.trust_opts.project_path.as_ref() {
            Some(p) => p.clone(),
            None => {
                let default_project = opts
                    .state
                    .projects
                    .default()
                    .context("A default project or project parameter is required.")?;
                default_project.path().clone()
            }
        };

        // Read (okta and authority) project parameters from project.json
        project_as_string = tokio::fs::read_to_string(path).await.into_diagnostic()?;
        serde_json::from_str(&project_as_string).into_diagnostic()?
    };

    let project: Project = (&proj).into();

    let trust_context_name = if let Some(trust_context_name) = &cmd.new_trust_context_name {
        trust_context_name
    } else {
        &project.name
    };

    if !cmd.force {
        if let Ok(trust_context) = opts.state.trust_contexts.get(trust_context_name) {
            if trust_context.config().id() != project.id {
                return Err(miette!(
                    "A trust context with the name {} already exists and is associated with a different project. Please choose a different name.",
                    trust_context_name
                ).into());
            }
        }
    }

    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;

    opts.state
        .trust_contexts
        .overwrite(trust_context_name, project.clone().try_into()?)?;

    Ok(project)
}
