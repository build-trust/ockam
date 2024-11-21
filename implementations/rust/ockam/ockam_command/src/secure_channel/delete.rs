use std::str::FromStr;

use clap::Parser;
use colorful::Colorful;
use serde_json::json;

use ockam::{route, Context};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::secure_channel::DeleteSecureChannelResponse;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::ReverseLocalConverter;
use ockam_core::{Address, AddressParseError};

use crate::util::async_cmd;
use crate::util::{api, exitcode};
use crate::{docs, CommandGlobalOpts};

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
    #[arg(value_name = "NODE", long, display_order = 800, value_parser = extract_address_value)]
    at: Option<String>,

    /// Address at which the channel to be deleted is running
    #[arg(value_parser(parse_address), display_order = 800)]
    address: Address,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "secure-channel delete".into()
    }

    fn print_output(
        &self,
        node_name: &String,
        address: &Address,
        options: &CommandGlobalOpts,
        response: DeleteSecureChannelResponse,
    ) -> miette::Result<()> {
        match response.channel {
            Some(address) => {
                let route = &route![address];
                match ReverseLocalConverter::convert_route(route) {
                    Ok(multiaddr) => {
                        // if stdout is not interactive/tty write the secure channel address to it
                        // in case some other program is trying to read it as piped input
                        if !options.terminal.is_tty() {
                            println!("{multiaddr}")
                        }

                        // if output format is json, write json to stdout.
                        if options.global_args.output_format().is_json() {
                            let json = json!([{ "address": multiaddr.to_string() }]);
                            println!("{json}");
                        }

                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if options.terminal.is_tty()
                            && !options.global_args.quiet
                            && options.global_args.output_format().is_json()
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
                    Err(_err) => {
                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if options.terminal.is_tty()
                            && !options.global_args.quiet
                            && options.global_args.output_format().is_plain()
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
                if options.terminal.is_tty()
                    && !options.global_args.quiet
                    && options.global_args.output_format().is_plain()
                {
                    eprintln!(
                        "Could not find secure channel with address {} at node {}",
                        address, &node_name
                    );
                }

                println!("channel with address {address} not found")
            }
        }
        Ok(())
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.terminal.confirmed_with_flag_or_prompt(
            self.yes,
            "Are you sure you want to delete this secure channel?",
        )? {
            let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
            let address = &self.address;
            let response: DeleteSecureChannelResponse =
                node.ask(ctx, api::delete_secure_channel(address)).await?;
            self.print_output(&node.node_name(), address, &opts, response)?;
        }
        Ok(())
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
