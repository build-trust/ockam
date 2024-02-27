use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::project::models::OktaAuth0;
use ockam_api::cloud::project::Project;
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::enroll::okta_oidc_provider::OktaOidcProvider;
use ockam_api::nodes::InMemoryNode;

use crate::{enroll::OidcServiceExt, fmt_ok, output::OutputFormat, terminal::color_primary};
use crate::{fmt_log, output::CredentialAndPurposeKeyDisplay};

use crate::util::api::{CloudOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/enroll/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/enroll/after_long_help.txt");

/// Use an enrollment ticket, or Okta, to enroll an identity with a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    /// Path to an enrollment ticket file, or hex encoded enrollment ticket
    #[arg(display_order = 800, group = "authentication_method", value_name = "ENROLLMENT TICKET PATH | ENROLLMENT TICKET", value_parser = parse_enroll_ticket)]
    pub enroll_ticket: Option<EnrollmentTicket>,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    /// Trust options, defaults to the default project
    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Use Okta instead of an enrollment ticket
    #[arg(display_order = 900, long = "okta", group = "authentication_method")]
    pub okta: bool,
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "enroll".into()
    }

    pub async fn async_run(self, ctx: &Context, mut opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.global_args.output_format == OutputFormat::Json {
            return Err(miette::miette!(
                "This command does not support JSON output. Please try running it again without '--output json'."
            ));
        }

        let identity = opts
            .state
            .get_named_identity_or_default(&self.cloud_opts.identity)
            .await?;
        let project = store_project(&opts, &self).await?;

        // Create secure channel to the project's authority node
        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;
        let authority_node_client = node
            .create_authority_client(
                &project.authority_identifier().into_diagnostic()?,
                project.authority_multiaddr().into_diagnostic()?,
                Some(identity.name()),
                None,
            )
            .await?;

        // Enroll
        if let Some(tkn) = self.enroll_ticket.as_ref() {
            authority_node_client
                .present_token(ctx, &tkn.one_time_code)
                .await?;
        } else if self.okta {
            // Get auth0 token
            let okta_config: OktaAuth0 = project
                .model()
                .okta_config
                .clone()
                .ok_or(miette!("Okta addon not configured"))?
                .into();

            let auth0 = OidcService::new(Arc::new(OktaOidcProvider::new(okta_config)));
            let token = auth0.get_token_interactively(&opts).await?;
            authority_node_client
                .enroll_with_oidc_token_okta(ctx, token)
                .await?;
        };

        // Issue credential
        let credential = authority_node_client.issue_credential(ctx).await?;

        // Get the project name to display to the user.
        let project_name = {
            let project = opts
                .state
                .projects()
                .get_project_by_name_or_default(&self.trust_opts.project_name.clone())
                .await?;
            project.name().to_string()
        };

        // Display success message to stderr.
        opts.terminal.write_line(&fmt_ok!(
            "Successfully enrolled identity to the {} project.",
            color_primary(project_name)
        ))?;
        opts.terminal.write_line(&fmt_log!(
            "{}.",
            "The identity has the following credential in this project"
        ))?;
        opts.terminal.write_line(&fmt_log!(
            "{}.",
            "The attributes below are attested by the project's membership authority"
        ))?;

        // Output the credential and purpose keys to stdout.
        opts.terminal
            .stdout()
            .plain(CredentialAndPurposeKeyDisplay(credential))
            .write_line()?;
        Ok(())
    }
}

async fn store_project(opts: &CommandGlobalOpts, cmd: &EnrollCommand) -> Result<Project> {
    // Retrieve project info from the enrollment ticket or project.json in the case of okta auth
    let project = if let Some(ticket) = &cmd.enroll_ticket {
        let project = ticket
            .project
            .as_ref()
            .expect("Enrollment ticket is invalid. Ticket does not contain a project.")
            .clone();
        opts.state
            .projects()
            .import_and_store_project(project)
            .await?
    } else {
        // OKTA AUTHENTICATION FLOW | PREVIOUSLY ENROLLED FLOW
        // currently okta auth does not use an enrollment token
        // however, it could be worked to use one in the future
        //
        // REQUIRES Project passed or default project
        opts.state
            .projects().get_project_by_name_or_default(&cmd.trust_opts.project_name)
            .await
            .context("A default project or project parameter is required. Run 'ockam project list' to get a list of available projects. You might also need to pass an enrollment ticket or path to the command.")?
    };

    Ok(project)
}
