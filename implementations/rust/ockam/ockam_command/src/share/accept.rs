use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::debug;

use ockam::Context;
use ockam_api::cloud::share::{AcceptInvitation, AcceptedInvitation};

use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
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

impl From<AcceptCommand> for AcceptInvitation {
    fn from(val: AcceptCommand) -> Self {
        Self { id: val.id }
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
    let mut rpc = Rpc::embedded(ctx, &opts).await?;

    let get_accepted_invitation = async {
        let req = cmd.into();
        debug!(?req);
        let invitation: AcceptedInvitation = rpc.ask(api::share::accept(req)).await?;
        *is_finished.lock().await = true;
        Ok(invitation)
    };

    let output_messages = vec![format!("Accepting share invitation...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (accepted, _) = try_join!(get_accepted_invitation, progress_output)?;

    delete_embedded_node(&opts, rpc.node_name()).await;

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
