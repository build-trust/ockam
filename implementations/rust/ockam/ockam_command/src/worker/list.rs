use clap::Args;
use colorful::Colorful;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::workers::WorkerList;
use ockam_api::nodes::BackgroundNodeClient;

use crate::{docs, CommandGlobalOpts};

use crate::util::api;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List workers on a node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Node at which to lookup workers
    #[arg(value_name = "NODE_NAME", long, display_order = 800, value_parser = extract_address_value)]
    at: Option<String>,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "workers list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_workers = async {
            let workers: WorkerList = node.ask(ctx, api::list_workers()).await?;
            *is_finished.lock().await = true;
            Ok(workers)
        };

        let output_messages = vec![format!(
            "Listing Workers on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (workers, _) = try_join!(get_workers, progress_output)?;

        let list = opts.terminal.build_list(
            &workers.list,
            &format!("No workers found on {}.", node.node_name()),
        )?;
        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}
