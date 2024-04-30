use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::colors::OckamColor;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};

use crate::util::async_cmd;

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Inlets on the default node
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "tcp-inlet list".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_inlets = async {
            let inlets: Vec<InletStatus> = node.ask(ctx, Request::get("/node/inlet")).await?;
            *is_finished.lock().await = true;
            Ok(inlets)
        };

        let output_messages = vec![format!(
            "Listing TCP Inlets on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (inlets, _) = try_join!(get_inlets, progress_output)?;

        let plain = opts.terminal.build_list(
            &inlets,
            "Inlets",
            &format!("No TCP Inlets found on {}", node.node_name()),
        )?;
        let json = serde_json::to_string(&inlets).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
