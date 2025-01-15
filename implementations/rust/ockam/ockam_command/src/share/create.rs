use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::debug;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::share::{Invitations, RoleInShare, ShareScope};

use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct CreateCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    #[arg(value_parser = clap::value_parser!(ShareScope))]
    pub scope: ShareScope,
    pub target_id: String,
    #[arg(value_parser = EmailAddress::parse)]
    pub recipient_email: EmailAddress,
    #[arg(default_value_t = RoleInShare::Admin, long, short = 'R', value_parser = clap::value_parser!(RoleInShare))]
    pub grant_role: RoleInShare,
    #[arg(long, short = 'x')]
    pub expires_at: Option<String>,
}

impl CreateCommand {
    pub fn name(&self) -> String {
        "create invitation".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let get_sent_invitation = async {
            let invitation = controller
                .create_invitation(
                    ctx,
                    self.expires_at.clone(),
                    self.grant_role.clone(),
                    self.recipient_email.clone(),
                    None,
                    self.scope.clone(),
                    self.target_id.clone(),
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
            "Invite {} to {} {} created, expiring at {}. {} will be notified via email.",
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
