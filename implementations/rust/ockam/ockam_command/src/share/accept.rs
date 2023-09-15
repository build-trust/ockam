use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cloud::share::Invitations;

use crate::node::util::LocalNode;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct AcceptCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    pub id: String,
}

impl AcceptCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AcceptCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: AcceptCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node = LocalNode::make(ctx, &opts, None).await?;

    let get_accepted_invitation = async {
        let invitation = node
            .accept_invitation(ctx, cmd.id)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()?;
        *is_finished.lock().await = true;
        Ok(invitation)
    };

    let output_messages = vec![format!("Accepting share invitation...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (accepted, _) = try_join!(get_accepted_invitation, progress_output)?;

    let plain = format!(
        "Accepted invite {} for {} {}",
        accepted.id, accepted.scope, accepted.target_id
    );
    let json = serde_json::to_string_pretty(&accepted).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
