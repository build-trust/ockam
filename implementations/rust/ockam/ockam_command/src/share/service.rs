use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::debug;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::share::{CreateServiceInvitation, Invitations};

use crate::shared_args::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ServiceCreateCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    pub project_id: String,
    #[arg(value_parser = EmailAddress::parse)]
    pub recipient_email: EmailAddress,

    pub project_identity: Identifier,
    pub project_route: String,
    pub project_authority_identity: Identifier,
    pub project_authority_route: String,
    pub shared_node_identity: Identifier,
    pub shared_node_route: String,

    pub enrollment_ticket: String,

    #[arg(long, short = 'x')]
    pub expires_at: Option<String>,
}

impl ServiceCreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create shared service".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let get_sent_invitation = async {
            let invitation = controller
                .create_service_invitation(
                    ctx,
                    self.expires_at.clone(),
                    self.project_id.clone(),
                    self.recipient_email.clone(),
                    self.project_identity.clone(),
                    self.project_route.clone(),
                    self.project_authority_identity.clone(),
                    self.project_authority_route.clone(),
                    self.shared_node_identity.clone(),
                    self.shared_node_route.clone(),
                    self.enrollment_ticket.clone(),
                )
                .await?;
            *is_finished.lock().await = true;
            Ok(invitation)
        };

        let output_messages = vec![format!("Creating invitation...\n",)];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (sent, _) = try_join!(get_sent_invitation, progress_output)?;

        debug!(?sent);

        let plain = fmt_ok!(
            "Invitation {} to {} {} created, expiring at {}. {} will be notified via email.",
            sent.id,
            sent.scope,
            sent.target_id,
            sent.expires_at,
            sent.recipient_email
        );
        let json = serde_json::to_string(&sent).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}

impl From<ServiceCreateCommand> for CreateServiceInvitation {
    fn from(val: ServiceCreateCommand) -> Self {
        let ServiceCreateCommand {
            expires_at,
            project_id,
            recipient_email,

            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,

            enrollment_ticket,
            ..
        } = val;
        Self {
            expires_at,
            project_id,
            recipient_email,

            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,

            enrollment_ticket,
        }
    }
}
