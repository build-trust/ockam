use std::fmt::Write;

use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::{route, Address, Result};

use crate::{docs, util::api, CommandGlobalOpts};
use ockam_api::output::Output;
use ockam_api::ReverseLocalConverter;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    /// Node at which the returned secure channels were initiated
    #[arg(value_name = "NODE_NAME", long, display_order = 800)]
    at: Option<String>,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "secure-channel list".into()
    }

    fn build_output(
        &self,
        node_name: &str,
        channel_address: &str,
        show_response: ShowSecureChannelResponse,
    ) -> crate::Result<SecureChannelListOutput> {
        let from = node_name.to_string();
        let at = {
            let channel_route = &route![channel_address];
            let channel_multiaddr = ReverseLocalConverter::convert_route(channel_route)
                .map_err(|_| miette!("Failed to convert route {channel_route} to multi-address"))?;
            channel_multiaddr.to_string()
        };

        let to = {
            let show_route = show_response.route.ok_or(miette!(
                "Failed to retrieve route from show channel response"
            ))?;
            show_route
                .split(" => ")
                .map(|p| {
                    let r = route![p];
                    ReverseLocalConverter::convert_route(&r)
                        .map_err(|_| miette!("Failed to convert route {r} to multi-address"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("")
        };

        Ok(SecureChannelListOutput { from, to, at })
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;

        let is_finished: Mutex<bool> = Mutex::new(false);
        let get_secure_channel_identifiers = async {
            let secure_channel_identifiers: Vec<String> =
                node.ask(ctx, api::list_secure_channels()).await?;
            *is_finished.lock().await = true;
            Ok(secure_channel_identifiers)
        };

        let output_messages = vec!["Retrieving secure channel identifiers...\n".to_string()];
        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (channel_identifiers, _) = try_join!(get_secure_channel_identifiers, progress_output)?;

        let mut responses = Vec::with_capacity(channel_identifiers.len());
        for channel_addr in &channel_identifiers {
            let is_finished: Mutex<bool> = Mutex::new(false);
            let get_secure_channel_output = async {
                let request = api::show_secure_channel(&Address::from(channel_addr));
                let show_response: ShowSecureChannelResponse = node.ask(ctx, request).await?;
                let secure_channel_output =
                    self.build_output(node.node_name(), channel_addr, show_response)?;
                *is_finished.lock().await = true;
                Ok(secure_channel_output)
            };
            let output_messages = vec![format!(
                "Retrieving secure channel {}...\n",
                channel_addr
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            )];
            let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

            let (secure_channel_output, _) = try_join!(get_secure_channel_output, progress_output)?;

            responses.push(secure_channel_output);
        }

        let list = opts.terminal.build_list(
            &responses,
            &format!("No secure channels found on {}", node.node_name()),
        )?;
        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}

pub struct SecureChannelListOutput {
    pub from: String,
    pub to: String,
    pub at: String,
}

impl Output for SecureChannelListOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "From {} to {} ",
            self.from
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.to
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "At {}",
            self.at
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;

        Ok(output)
    }
}
