use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::share::Invitations;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
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

#[derive(Clone)]
struct ShowNodeCommand {
    opts: CommandGlobalOpts,
    command: ShowCommand,
}

impl ShowNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ShowCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ShowNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let controller = node.create_controller().await?;

        let get_invitation_with_access = async {
            let invitation_with_access = controller
                .show_invitation(node.ctx(), self.command.invitation_id.clone())
                .await?;
            *is_finished.lock().await = true;
            Ok(invitation_with_access)
        };

        let output_messages = vec!["Showing invitation...\n".to_string()];

        let progress_output = self
            .opts
            .terminal
            .loop_messages(&output_messages, &is_finished);

        let (response, _) = try_join!(get_invitation_with_access, progress_output)?;

        // TODO: Emit connection details
        let plain = fmt_ok!("Invite {}", response.invitation.id);
        let json = serde_json::to_string(&response).into_diagnostic()?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "show invitation".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
