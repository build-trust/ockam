use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::debug;

use ockam::Context;
use ockam_api::cloud::share::{Invitations, RoleInShare, ShareScope};

use ockam_api::nodes::InMemoryNode;

use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct CreateCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    #[arg(value_parser = clap::value_parser!(ShareScope))]
    pub scope: ShareScope,
    pub target_id: String,
    pub recipient_email: String,
    #[arg(default_value_t = RoleInShare::Admin, long, short = 'R', value_parser = clap::value_parser!(RoleInShare))]
    pub grant_role: RoleInShare,
    #[arg(long, short = 'x')]
    pub expires_at: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let get_sent_invitation = async {
        let invitation = controller
            .create_invitation(
                ctx,
                cmd.expires_at,
                cmd.grant_role,
                cmd.recipient_email,
                None,
                cmd.scope,
                cmd.target_id,
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
        "Invite {} to {} {} created, expiring at {}. {} will be notified via email.",
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
