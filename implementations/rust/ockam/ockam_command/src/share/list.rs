use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cloud::share::{InvitationList, InvitationListKind};

use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ListCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    // #[arg(long, short, value_parser = clap::value_parser!(InvitationListKind))]
    // pub kind: InvitationListKind,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    _cmd: ListCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let mut rpc = Rpc::embedded(ctx, &opts).await?;

    let get_invitations = async {
        let invitations: InvitationList =
            rpc.ask(api::share::list(InvitationListKind::All)).await?;
        *is_finished.lock().await = true;
        Ok(invitations)
    };

    let output_messages = vec![format!("Listing shares...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (shares, _) = try_join!(get_invitations, progress_output)?;

    if let Some(sent) = shares.sent.as_ref() {
        let opts = opts.clone();
        let plain = opts
            .terminal
            .build_list(sent, "Sent Shares", "No sent shares found.")?;
        let json = serde_json::to_string_pretty(sent).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
    }

    if let Some(received) = shares.received.as_ref() {
        let opts = opts.clone();
        let plain =
            opts.terminal
                .build_list(received, "Received Shares", "No received shares found.")?;
        let json = serde_json::to_string_pretty(received).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
    }

    delete_embedded_node(&opts, rpc.node_name()).await;

    Ok(())
}
