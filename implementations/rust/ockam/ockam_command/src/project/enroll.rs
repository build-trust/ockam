use std::fmt::{Debug, Formatter, Write};
use std::sync::Arc;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::identity::models::CredentialData;
use ockam::Context;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::project::models::OktaAuth0;
use ockam_api::cloud::project::Project;
use ockam_api::colors::color_primary;
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::enroll::okta_oidc_provider::OktaOidcProvider;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::human_readable_time;
use ockam_api::terminal::fmt;
use ockam_api::{fmt_log, fmt_ok};

use crate::enroll::OidcServiceExt;
use crate::shared_args::{IdentityOpts, RetryOpts, TrustOpts};
use crate::value_parsers::parse_enrollment_ticket;
use crate::{docs, Command, CommandGlobalOpts, Error, Result};

const LONG_ABOUT: &str = include_str!("./static/enroll/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/enroll/after_long_help.txt");

/// Use an enrollment ticket, or Okta, to enroll an identity with a project
#[derive(Clone, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    /// Path, URL or inlined hex-encoded enrollment ticket
    #[arg(display_order = 800, group = "authentication_method", value_name = "ENROLLMENT TICKET", value_parser = parse_enrollment_ticket)]
    pub enrollment_ticket: Option<EnrollmentTicket>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,

    /// Trust options, defaults to the default project
    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Use Okta instead of an enrollment ticket
    #[arg(display_order = 900, long = "okta", group = "authentication_method")]
    pub okta: bool,

    #[command(flatten)]
    pub retry_opts: RetryOpts,
}

/// This custom Debug instance hides the enrollment ticket
impl Debug for EnrollCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrollCommand")
            .field("identity_opts", &self.identity_opts)
            .field("trust_opts", &self.trust_opts)
            .field("okta", &self.okta)
            .field("retry_opts", &self.retry_opts)
            .finish()
    }
}

#[async_trait]
impl Command for EnrollCommand {
    const NAME: &'static str = "project enroll";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        if opts.global_args.output_format()?.is_json() {
            return Err(miette::miette!(
                "This command does not support JSON output. Please try running it again without '--output json'."
            ).into());
        }

        let identity = opts
            .state
            .get_named_identity_or_default(&self.identity_opts.identity_name)
            .await?;
        let project = self.store_project(&opts).await?;

        // Create secure channel to the project's authority node
        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;
        let authority_node_client = node
            .create_authority_client(&project, Some(identity.name()))
            .await?;

        // Enroll
        if let Some(tkn) = self.enrollment_ticket.as_ref() {
            match authority_node_client
                .present_token(ctx, &tkn.one_time_code)
                .await?
            {
                EnrollStatus::EnrolledSuccessfully => {}
                EnrollStatus::AlreadyEnrolled => {
                    opts.terminal
                        .write_line(&fmt_ok!("Identity is already enrolled with the project"))?;
                    return Ok(());
                }
                EnrollStatus::FailedNoStatus(msg) => {
                    return Err(Error::Retry(miette!(
                        "Failed to enroll identity with project. {msg}"
                    )))
                }
                EnrollStatus::UnexpectedStatus(msg, status) => {
                    return Err(Error::Retry(miette!(
                        "Failed to enroll identity with project. {msg} {status}"
                    )))
                }
            }
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
                .await
                .map_err(Error::Retry)?;
        };

        // Issue credential
        let credential = authority_node_client
            .issue_credential(ctx)
            .await
            .map_err(Error::Retry)?
            .get_credential_data()
            .into_diagnostic()
            .wrap_err("Failed to decode the credential received from the project authority")?;

        // Get the project name to display to the user.
        let project_name = {
            let project = opts
                .state
                .projects()
                .get_project_by_name_or_default(&self.trust_opts.project_name.clone())
                .await?;
            project.name().to_string()
        };

        // Output
        let plain = self.plain_output(&identity.name(), &project_name, &credential)?;
        opts.terminal.clone().stdout().plain(plain).write_line()?;

        Ok(())
    }
}

impl EnrollCommand {
    async fn store_project(&self, opts: &CommandGlobalOpts) -> Result<Project> {
        // Retrieve project info from the enrollment ticket or project.json in the case of okta auth
        let project = if let Some(ticket) = &self.enrollment_ticket {
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
                .projects().get_project_by_name_or_default(&self.trust_opts.project_name)
                .await
                .context("A default project or project parameter is required. Run 'ockam project list' to get a list of available projects. You might also need to pass an enrollment ticket or path to the command.")?
        };

        Ok(project)
    }

    fn plain_output(
        &self,
        identity_name: &str,
        project_name: &str,
        credential: &CredentialData,
    ) -> Result<String> {
        let mut buf = String::new();
        writeln!(
            buf,
            "{}",
            fmt_ok!(
                "Successfully enrolled identity {} to the {} project.\n",
                color_primary(identity_name),
                color_primary(project_name)
            )
        )?;

        writeln!(
            buf,
            "{}",
            fmt_log!("The identity has a credential in this project")
        )?;
        writeln!(
            buf,
            "{}",
            fmt_log!(
                "created at {} that expires at {}\n",
                color_primary(human_readable_time(credential.created_at)),
                color_primary(human_readable_time(credential.expires_at))
            )
        )?;

        if !credential.subject_attributes.map.is_empty() {
            writeln!(
                buf,
                "{}",
                fmt_log!(
                    "The following attributes are attested by the project's membership authority:"
                )
            )?;
            for (k, v) in credential.subject_attributes.map.iter() {
                let k = std::str::from_utf8(k).unwrap_or("**binary**");
                let v = std::str::from_utf8(v).unwrap_or("**binary**");
                writeln!(
                    buf,
                    "{}",
                    fmt_log!(
                        "{}{}",
                        fmt::INDENTATION,
                        color_primary(format!("\"{k}={v}\""))
                    )
                )?;
            }
        }
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(EnrollCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }
}
