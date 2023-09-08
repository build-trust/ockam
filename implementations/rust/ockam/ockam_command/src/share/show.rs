use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cloud::share::InvitationWithAccess;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ShowCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let mut rpc = Rpc::embedded(ctx, &opts).await?;

    let get_invitation_with_access = async {
        let invitation_with_access: InvitationWithAccess =
            rpc.ask(api::share::show(cmd.invitation_id)).await?;
        *is_finished.lock().await = true;
        Ok(invitation_with_access)
    };

    let output_messages = vec![format!("Showing invitation...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (response, _) = try_join!(get_invitation_with_access, progress_output)?;

    delete_embedded_node(&opts, rpc.node_name()).await;

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
