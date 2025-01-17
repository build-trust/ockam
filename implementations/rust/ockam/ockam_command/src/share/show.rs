use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::share::Invitations;

use crate::shared_args::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ShowCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    pub invitation_id: String,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show invitation".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let get_invitation_with_access = async {
            let invitation_with_access = controller
                .show_invitation(ctx, self.invitation_id.clone())
                .await?;
            *is_finished.lock().await = true;
            Ok(invitation_with_access)
        };

        let output_messages = vec![format!("Showing invitation...\n",)];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (response, _) = try_join!(get_invitation_with_access, progress_output)?;

        // TODO: Emit connection details
        let plain = fmt_ok!("Invite {}", response.invitation.id);
        let json = serde_json::to_string(&response).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
