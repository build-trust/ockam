use crate::{
    util::{api, exitcode, get_final_element, node_rpc, Rpc},
    CommandGlobalOpts, OutputFormat, Result,
};

use atty::Stream;
use clap::Args;
use colorful::Colorful;
use serde_json::json;

use ockam::{route, Context};
use ockam_api::{nodes::models::secure_channel::DeleteSecureChannelResponse, route_to_multiaddr};
use ockam_core::Address;

/// Delete Secure Channels
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Node from which to initiate the secure channel (required)
    #[clap(value_name = "NODE", long, display_order = 800)]
    at: String,

    /// Address at which the channel to be deleted is running (required)
    #[clap(display_order = 800)]
    address: Address,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    // Read the `at` argument and return node name
    fn parse_at_node(&self) -> String {
        get_final_element(&self.at).to_string()
    }

    fn print_output(
        &self,
        parsed_at: &String,
        address: &Address,
        options: &CommandGlobalOpts,
        response: DeleteSecureChannelResponse,
    ) {
        match response.channel {
            Some(address) => {
                let route = &route![address.to_string()];
                match route_to_multiaddr(route) {
                    Some(multiaddr) => {
                        // if stdout is not interactive/tty write the secure channel address to it
                        // in case some other program is trying to read it as piped input
                        if !atty::is(Stream::Stdout) {
                            println!("{}", multiaddr)
                        }

                        // if output format is json, write json to stdout.
                        if options.global_args.output_format == OutputFormat::Json {
                            let json = json!([{ "address": multiaddr.to_string() }]);
                            println!("{}", json);
                        }

                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if atty::is(Stream::Stderr)
                            && !options.global_args.quiet
                            && options.global_args.output_format == OutputFormat::Plain
                        {
                            if options.global_args.no_color {
                                eprintln!("\n  Deleted Secure Channel:");
                                eprintln!("  •        At: /node/{}", &self.at);
                                eprintln!("  •   Address: {}", &self.address);
                            } else {
                                eprintln!("\n  Deleted Secure Channel:");

                                // At:
                                eprintln!("{}", "  •        At: ".light_magenta());
                                eprintln!("{}", format!("/node/{}", parsed_at).light_yellow());

                                // Address:
                                eprintln!("{}", "  •   Address: ".light_magenta());
                                eprintln!("{}", &self.address.to_string().light_yellow());
                            }
                        }
                    }
                    None => {
                        // if stderr is interactive/tty and we haven't been asked to be quiet
                        // and output format is plain then write a plain info to stderr.
                        if atty::is(Stream::Stderr)
                            && !options.global_args.quiet
                            && options.global_args.output_format == OutputFormat::Plain
                        {
                            eprintln!(
                                "Could not convert returned secure channel route {} into a multiaddr",
                                route
                            );
                        }

                        // return the exitcode::PROTOCOL since if things are going as expected
                        // a route in the response should be convertable to multiaddr.
                        std::process::exit(exitcode::PROTOCOL);
                    }
                }
            }
            None => {
                // if stderr is interactive/tty and we haven't been asked to be quiet
                // and output format is plain then write a plain info to stderr.
                if atty::is(Stream::Stderr)
                    && !options.global_args.quiet
                    && options.global_args.output_format == OutputFormat::Plain
                {
                    eprintln!(
                        "Could not find secure channel with address {} at node {}",
                        address, &self.at
                    );
                }

                println!("channel with address {} not found", address)
            }
        }
    }
}

async fn rpc(ctx: Context, (options, command): (CommandGlobalOpts, DeleteCommand)) -> Result<()> {
    let at = &command.parse_at_node();
    let address = &command.address;

    let mut rpc = Rpc::new(&ctx, &options, at)?;
    let request = api::delete_secure_channel(address);
    rpc.request(request).await?;
    let response = rpc.parse_response::<DeleteSecureChannelResponse>()?;

    command.print_output(at, address, &options, response);

    Ok(())
}
