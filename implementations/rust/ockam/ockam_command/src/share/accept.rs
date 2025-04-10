use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
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
pub struct AcceptCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    pub id: String,
}

#[derive(Clone)]
struct AcceptNodeCommand {
    opts: CommandGlobalOpts,
    command: AcceptCommand,
}

impl AcceptNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: AcceptCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for AcceptNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let controller = node.create_controller().await?;

        let get_accepted_invitation = async {
            let invitation = controller
                .accept_invitation(node.ctx(), self.command.id.clone())
                .await?;
            *is_finished.lock().await = true;
            Ok(invitation)
        };

        let output_messages = vec!["Accepting share invitation...\n".to_string()];

        let progress_output = self
            .opts
            .terminal
            .loop_messages(&output_messages, &is_finished);

        let (accepted, _) = try_join!(get_accepted_invitation, progress_output)?;

        let plain = format!(
            "Accepted invite {} for {} {}",
            accepted.id, accepted.scope, accepted.target_id
        );
        let json = serde_json::to_string(&accepted).into_diagnostic()?;
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

impl AcceptCommand {
    pub fn name(&self) -> String {
        "accept invitation".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        AcceptNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
