use std::str::FromStr;

use clap::Parser;
use colorful::Colorful;
use serde_json::json;

use ockam::{route, Context};
use ockam_api::nodes::BackgroundNode;
use ockam_api::{nodes::models::secure_channel::DeleteSecureChannelResponse, route_to_multiaddr};
use ockam_core::{Address, AddressParseError};

use crate::docs;
use crate::node::get_node_name;
use crate::util::{is_tty, parse_node_name};
use crate::{
    util::{api, exitcode, node_rpc},
    CommandGlobalOpts, OutputFormat,
};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete Secure Channels
#[derive(Clone, Debug, Parser)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    /// Node at which the secure channel was initiated
    #[arg(value_name = "NODE", long, display_order = 800)]
    at: Option<String>,

    /// Address at which the channel to be deleted is running
    #[arg(value_parser(parse_address), display_order = 800)]
    address: Address,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    fn print_output(
        &self,
        node_name: &String,
        address: &Address,
        options: &CommandGlobalOpts,
        response: DeleteSecureChannelResponse,
    ) {
        match response.channel {
            Some(address) => {
                let route = &route![address];
                match route_to_multiaddr(route) {
                    Some(multiaddr) => {
                        // if stdout is not interactive/tty write the secure channel address to it
                        // in case some other program is trying to read it as piped input
                        if !is_tty(std::io::stdout()) {
                            println!("{multiaddr}")
                        }

                        // if output format is json, write json to stdout.
                        if options.global_args.output_format == OutputFormat::Json {
                            let json = json!([{ "address": multiaddr.to_string() }]);
                            println!("{json}");
                        }

                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if is_tty(std::io::stderr())
                            && !options.global_args.quiet
                            && options.global_args.output_format == OutputFormat::Plain
                        {
                            if options.global_args.no_color {
                                eprintln!("\n  Deleted Secure Channel:");
                                eprintln!("  •        At: /node/{}", &node_name);
                                eprintln!("  •   Address: {}", &self.address);
                            } else {
                                eprintln!("\n  Deleted Secure Channel:");

                                // At:
                                eprintln!("{}", "  •        At: ".light_magenta());
                                eprintln!("{}", format!("/node/{node_name}").light_yellow());

                                // Address:
                                eprintln!("{}", "  •   Address: ".light_magenta());
                                eprintln!("{}", &self.address.to_string().light_yellow());
                            }
                        }
                    }
                    None => {
                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if is_tty(std::io::stderr())
                            && !options.global_args.quiet
                            && options.global_args.output_format == OutputFormat::Plain
                        {
                            eprintln!(
                                "Could not convert returned secure channel route {route} into a multiaddr"
                            );
                        }

                        // return the exitcode::PROTOCOL since if things are going as expected
                        // a route in the response should be convertible to multiaddr.
                        std::process::exit(exitcode::PROTOCOL);
                    }
                }
            }
            None => {
                // if stderr is interactive/tty and we haven't been asked to be quiet
                // and output format is plain then write a plain info to stderr.
                if is_tty(std::io::stderr())
                    && !options.global_args.quiet
                    && options.global_args.output_format == OutputFormat::Plain
                {
                    eprintln!(
                        "Could not find secure channel with address {} at node {}",
                        address, &node_name
                    );
                }

                println!("channel with address {address} not found")
            }
        }
    }
}

fn parse_address(input: &str) -> Result<Address, AddressParseError> {
    let buf: String = input.into();

    if buf.contains("/service/") {
        let service_vec: Vec<_> = buf.split('/').collect();
        // If /service/<some value> was passed, we will have split len greater than or equal to 3
        // ["", "service", "228003f018d277a7e53f15475d111052"]
        // we will pass index 2 to from_str
        // EG: /service/228003f018d277a7e53f15475d111052
        //       /service/228003f018d277a7e53f15475d111052/
        if service_vec.len() >= 3 && !service_vec[2].is_empty() {
            return Address::from_str(service_vec[2]);
        }
    }
    Address::from_str(&buf)
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this secure channel?",
    )? {
        let at = get_node_name(&opts.state, &cmd.at);
        let node_name = parse_node_name(&at)?;
        let address = &cmd.address;
        let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
        let response: DeleteSecureChannelResponse =
            node.ask(&ctx, api::delete_secure_channel(address)).await?;
        cmd.print_output(&node_name, address, &opts, response);
    }
    Ok(())
}
