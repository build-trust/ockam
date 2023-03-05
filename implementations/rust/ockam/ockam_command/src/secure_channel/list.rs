use clap::Args;
use colorful::Colorful;

use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::route_to_multiaddr;
use ockam_core::{route, Address};

use serde_json::json;

use crate::secure_channel::HELP_DETAIL;
use crate::util::{is_tty, RpcBuilder};
use crate::{
    exitcode, help,
    util::{api, node_rpc},
    CommandGlobalOpts, OutputFormat,
};

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct ListCommand {
    /// Node at which the returned secure channels were initiated (required)
    #[arg(value_name = "NODE", long, display_order = 800)]
    at: String,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }

    fn print_output(
        &self,
        options: &CommandGlobalOpts,
        channel_identifiers: Vec<String>,
        show_responses: Vec<ShowSecureChannelResponse>,
    ) -> Result<(), String> {
        let zipped = channel_identifiers.iter().zip(show_responses);

        if zipped.len() > 0 && has_plain_stderr(options) {
            println!("\nSecure Channels")
        }

        for (channel_address, show_response) in zipped {
            let from = &self.at;

            let at = {
                let channel_route = &route![channel_address];
                let channel_multiaddr = route_to_multiaddr(channel_route).ok_or(format!(
                    "Failed to convert route {channel_route} to multi-address"
                ))?;
                channel_multiaddr.to_string()
            };

            let to = {
                let show_route = show_response
                    .route
                    .ok_or("Failed to retrieve route from show channel response")?;
                show_route
                    .split(" => ")
                    .map(|p| {
                        let r = route![p];
                        route_to_multiaddr(&r)
                            .ok_or(format!("Failed to convert route {r} to multi-address"))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("")
            };

            // if stdout is not interactive/tty write the secure channel address to it
            // in case some other program is trying to read it as piped input
            if !is_tty(std::io::stdout()) {
                println!("{at}")
            }

            // if output format is json, write json to stdout.
            if options.global_args.output_format == OutputFormat::Json {
                let json = json!([{ "address": at }]);
                println!("{json}");
            }

            // if stderr is interactive/tty and we haven't been asked to be quiet
            // and output format is plain then write a plain info to stderr.
            if has_plain_stderr(options) {
                println!("\n    Secure Channel:");
                if options.global_args.no_color {
                    eprintln!("      • From: /node/{from}");
                    eprintln!("      •   To: {to}");
                    eprintln!("      •   At: {at}");
                } else {
                    // From:
                    eprint!("{}", "      • From: ".light_magenta());
                    eprintln!("{}", format!("/node/{from}").light_yellow());

                    // To:
                    eprint!("{}", "      •   To: ".light_magenta());
                    eprintln!("{}", to.light_yellow());

                    // At:
                    eprint!("{}", "      •   At: ".light_magenta());
                    eprintln!("{}", at.light_yellow());
                }
            }
        }
        Ok(())
    }
}

#[inline]
fn has_plain_stderr(options: &CommandGlobalOpts) -> bool {
    is_tty(std::io::stderr())
        && !options.global_args.quiet
        && options.global_args.output_format == OutputFormat::Plain
}

async fn rpc(
    ctx: Context,
    (options, command): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    // We need this TCPTransport handle to ensure that we are using the same transport across
    // multiple RPC calls. Creating a RPC instance without explicit transport results in a router
    // instance being registered for the same transport type multiple times which is not allowed
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &options, &command.at)
        .tcp(&tcp)?
        .build();
    rpc.request(api::list_secure_channels()).await?;
    let channel_identifiers = rpc.parse_response::<Vec<String>>()?;

    let mut response_rpcs = Vec::with_capacity(channel_identifiers.len());
    for channel_addr in &channel_identifiers {
        let mut rpc = RpcBuilder::new(&ctx, &options, &command.at)
            .tcp(&tcp)?
            .build();
        let request = api::show_secure_channel(&Address::from(channel_addr));
        rpc.request(request).await?;
        response_rpcs.push(rpc);
    }
    let results: Result<Vec<_>, _> = response_rpcs
        .iter()
        .map(|rpc| rpc.parse_response::<ShowSecureChannelResponse>())
        .into_iter()
        .collect();
    let responses = results?;

    if let Err(e) = command.print_output(&options, channel_identifiers, responses) {
        if is_tty(std::io::stderr())
            && !options.global_args.quiet
            && options.global_args.output_format == OutputFormat::Plain
        {
            eprintln!("{e}");
        }
        std::process::exit(exitcode::PROTOCOL)
    }

    Ok(())
}
