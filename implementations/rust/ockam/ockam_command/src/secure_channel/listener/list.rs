use clap::Args;
use colorful::Colorful;

use tokio::sync::Mutex;
use tokio::try_join;

use ockam::identity::SecureChannelListener;
use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::BackgroundNodeClient;

use crate::node::NodeOpts;
use crate::util::api;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Secure Channel Listeners
#[derive(Args, Clone, Debug)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    /// Node of which secure listeners shall be listed
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "secure-channel-listeners list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_listeners = async {
            let listeners: Vec<SecureChannelListener> =
                node.ask(ctx, api::list_secure_channel_listener()).await?;
            *is_finished.lock().await = true;
            Ok(listeners)
        };

        let output_messages = vec![format!(
            "Listing secure channel listeners on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (secure_channel_listeners, _) = try_join!(get_listeners, progress_output)?;

        let list = opts.terminal.build_list(
            &secure_channel_listeners,
            &format!(
                "No secure channel listeners found at node {}.",
                node.node_name()
            ),
        )?;
        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}
