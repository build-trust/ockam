use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::colors::{color_primary, OckamColor};
use ockam_api::nodes::models::secure_channel::{SecureChannelList, ShowSecureChannelResponse};
use ockam_api::nodes::BackgroundNodeClient;

use crate::{docs, util::api, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    /// Node at which the returned secure channels were initiated
    #[arg(value_name = "NODE_NAME", long, display_order = 800)]
    at: Option<String>,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "secure-channel list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let node = BackgroundNodeClient::create(ctx, opts.state.clone(), &self.at).await?;

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message("Retrieving secure channel addresses...");
        }
        let secure_channels_addresses: SecureChannelList =
            node.ask(ctx, api::list_secure_channels()).await?;
        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        let mut secure_channels = Vec::with_capacity(secure_channels_addresses.0.len());
        let spinner = opts.terminal.spinner();
        for secure_channel_address in &secure_channels_addresses.0 {
            if let Some(spinner) = &spinner {
                spinner.set_message(format!(
                    "Retrieving secure channel {}...\n",
                    secure_channel_address
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ));
            }
            let request = api::show_secure_channel(secure_channel_address);
            let res: ShowSecureChannelResponse = node.ask(ctx, request).await?;
            secure_channels.push(res);
        }

        let plain = opts.terminal.build_list(
            &secure_channels,
            &format!(
                "No secure channels found on {}",
                color_primary(node.node_name())
            ),
        )?;
        opts.terminal
            .to_stdout()
            .plain(plain)
            .json_obj(secure_channels)?
            .write_line()?;

        Ok(())
    }
}
