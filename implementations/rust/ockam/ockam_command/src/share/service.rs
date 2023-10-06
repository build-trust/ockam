use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::debug;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::cloud::share::{CreateServiceInvitation, Invitations};

use ockam_api::nodes::InMemoryNode;

use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ServiceCreateCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    pub project_id: String,
    pub recipient_email: String,

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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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

async fn rpc(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ServiceCreateCommand),
) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: ServiceCreateCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let get_sent_invitation = async {
        let invitation = controller
            .create_service_invitation(
                ctx,
                cmd.expires_at,
                cmd.project_id,
                cmd.recipient_email,
                cmd.project_identity,
                cmd.project_route,
                cmd.project_authority_identity,
                cmd.project_authority_route,
                cmd.shared_node_identity,
                cmd.shared_node_route,
                cmd.enrollment_ticket,
            )
            .await?;
        *is_finished.lock().await = true;
        Ok(invitation)
    };

    let output_messages = vec![format!("Creating invitation...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

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
    let json = serde_json::to_string_pretty(&sent).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
