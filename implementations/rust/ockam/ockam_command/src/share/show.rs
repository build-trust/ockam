use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cloud::share::Invitations;

use ockam_api::nodes::InMemoryNode;

use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ShowCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    pub invitation_id: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let get_invitation_with_access = async {
        let invitation_with_access = controller.show_invitation(ctx, cmd.invitation_id).await?;
        *is_finished.lock().await = true;
        Ok(invitation_with_access)
    };

    let output_messages = vec![format!("Showing invitation...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (response, _) = try_join!(get_invitation_with_access, progress_output)?;

    // TODO: Emit connection details
    let plain = fmt_ok!("Invite {}", response.invitation.id);
    let json = serde_json::to_string_pretty(&response).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
