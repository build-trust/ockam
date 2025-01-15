use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::share::Invitations;

use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct AcceptCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    pub id: String,
}

impl AcceptCommand {
    pub fn name(&self) -> String {
        "accept invitation".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let get_accepted_invitation = async {
            let invitation = controller.accept_invitation(ctx, self.id.clone()).await?;
            *is_finished.lock().await = true;
            Ok(invitation)
        };

        let output_messages = vec![format!("Accepting share invitation...\n",)];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (accepted, _) = try_join!(get_accepted_invitation, progress_output)?;

        let plain = format!(
            "Accepted invite {} for {} {}",
            accepted.id, accepted.scope, accepted.target_id
        );
        let json = serde_json::to_string(&accepted).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
