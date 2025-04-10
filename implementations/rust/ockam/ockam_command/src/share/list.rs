use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::share::{InvitationListKind, Invitations};

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
)]
pub struct ListCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    // #[arg(long, short, value_parser = clap::value_parser!(InvitationListKind))]
    // pub kind: InvitationListKind,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
}

impl ListNodeCommand {
    pub fn new(opts: CommandGlobalOpts) -> Self {
        Self { opts }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let controller = node.create_controller().await?;

        let get_invitations = async {
            let invitations = controller
                .list_invitations(node.ctx(), InvitationListKind::All)
                .await?;
            *is_finished.lock().await = true;
            Ok(invitations)
        };

        let output_messages = vec!["Listing shares...\n".to_string()];

        let progress_output = self
            .opts
            .terminal
            .loop_messages(&output_messages, &is_finished);

        let (shares, _) = try_join!(get_invitations, progress_output)?;

        if let Some(sent) = shares.sent.as_ref() {
            let plain = self
                .opts
                .terminal
                .build_list(sent, "No sent shares found.")?;
            let json = serde_json::to_string(sent).into_diagnostic()?;
            self.opts
                .terminal
                .clone()
                .to_stdout()
                .plain(plain)
                .json(json)
                .write_line()?;
        }

        if let Some(received) = shares.received.as_ref() {
            let plain = self
                .opts
                .terminal
                .build_list(received, "No received shares found.")?;
            let json = serde_json::to_string(received).into_diagnostic()?;
            self.opts
                .terminal
                .clone()
                .to_stdout()
                .plain(plain)
                .json(json)
                .write_line()?;
        }

        Ok(())
    }
}
impl ListCommand {
    pub fn name(&self) -> String {
        "list invitations".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ListNodeCommand::new(opts.clone())
            .execute(ctx, opts.state)
            .await
    }
}
