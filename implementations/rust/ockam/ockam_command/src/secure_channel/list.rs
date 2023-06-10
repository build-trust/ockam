use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;
use std::fmt::Write;

use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::route_to_multiaddr;
use ockam_core::{route, Address};

use tokio::sync::Mutex;
use tokio::try_join;

use crate::terminal::OckamColor;
use crate::util::output::Output;
use crate::util::{extract_address_value, RpcBuilder};
use crate::{
    docs,
    util::{api, node_rpc},
    CommandGlobalOpts,
};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    /// Node at which the returned secure channels were initiated (required)
    #[arg(value_name = "NODE_NAME", long, display_order = 800)]
    at: String,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }

    fn build_output(
        &self,
        channel_address: &str,
        show_response: ShowSecureChannelResponse,
    ) -> crate::Result<SecureChannelListOutput> {
        let from = format!("/node/{}", &self.at);
        let at = {
            let channel_route = &route![channel_address];
            let channel_multiaddr = route_to_multiaddr(channel_route).ok_or(anyhow!(
                "Failed to convert route {channel_route} to multi-address"
            ))?;
            channel_multiaddr.to_string()
        };

        let to = {
            let show_route = show_response.route.ok_or(anyhow!(
                "Failed to retrieve route from show channel response"
            ))?;
            show_route
                .split(" => ")
                .map(|p| {
                    let r = route![p];
                    route_to_multiaddr(&r)
                        .ok_or(anyhow!("Failed to convert route {r} to multi-address"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("")
        };

        Ok(SecureChannelListOutput { from, to, at })
    }
}

async fn rpc(
    ctx: Context,
    (options, command): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    // We need this TCPTransport handle to ensure that we are using the same transport across
    // multiple RPC calls. Creating a RPC instance without explicit transport results in a router
    // instance being registered for the same transport type multiple times which is not allowed
    let tcp = TcpTransport::create(&ctx).await?;
    let node_name = extract_address_value(&command.at)?;

    let is_finished: Mutex<bool> = Mutex::new(false);
    let mut rpc = RpcBuilder::new(&ctx, &options, &node_name)
        .tcp(&tcp)?
        .build();

    let send_req = async {
        rpc.request(api::list_secure_channels()).await?;
        let channel_identifiers = rpc.parse_response::<Vec<String>>()?;

        *is_finished.lock().await = true;
        Ok(channel_identifiers)
    };

    let output_messages = vec!["Retrieving secure channel identifiers...\n".to_string()];
    let progress_output = options
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (channel_identifiers, _) = try_join!(send_req, progress_output)?;

    let mut responses = Vec::with_capacity(channel_identifiers.len());
    for channel_addr in &channel_identifiers {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let mut rpc = RpcBuilder::new(&ctx, &options, &command.at)
            .tcp(&tcp)?
            .build();
        let send_req = async {
            let request: ockam_core::api::RequestBuilder<
                ockam_api::nodes::models::secure_channel::ShowSecureChannelRequest,
            > = api::show_secure_channel(&Address::from(channel_addr));
            rpc.request(request).await?;
            let show_response = rpc.parse_response::<ShowSecureChannelResponse>()?;
            let secure_channel_output = command.build_output(channel_addr, show_response)?;
            *is_finished.lock().await = true;
            crate::Result::Ok(secure_channel_output)
        };
        let output_messages = vec![format!(
            "Retrieving secure channel {}...\n",
            channel_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = options
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (secure_channel_output, _) = try_join!(send_req, progress_output)?;

        responses.push(secure_channel_output);
    }

    let list = options.terminal.build_list(
        &responses,
        &format!("Secure Channels on {}", node_name),
        &format!("No secure channels found on {}", node_name),
    )?;
    options.terminal.stdout().plain(list).write_line()?;

    Ok(())
}

pub struct SecureChannelListOutput {
    pub from: String,
    pub to: String,
    pub at: String,
}

impl Output for SecureChannelListOutput {
    fn output(&self) -> crate::Result<String> {
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
