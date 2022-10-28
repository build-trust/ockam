use crate::{
    util::{api, extract_address_value, node_rpc, Rpc},
    CommandGlobalOpts, OutputFormat,
};
use anyhow::Context;
use clap::Args;
use colorful::Colorful;
use ockam::{route, Route, TCP};
use ockam_api::{nodes::models, route_to_multiaddr};
use serde_json::json;
use std::net::SocketAddrV4;

#[derive(Clone, Debug, Args)]
pub struct TcpConnectionNodeOpts {
    /// Node that will initiate the connection
    #[arg(
        global = true,
        short,
        long,
        value_name = "NODE",
        default_value = "default"
    )]
    pub from: String,
}

#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TcpConnectionNodeOpts,

    /// The address to connect to (required)
    #[arg(id = "to", short, long, value_name = "ADDRESS")]
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }

    fn print_output(
        &self,
        node_name: &str,
        options: &CommandGlobalOpts,
        response: &models::transport::TransportStatus,
    ) -> crate::Result<()> {
        // if output format is json, write json to stdout.
        match options.global_args.output_format {
            OutputFormat::Plain => {
                let from = &self.node_opts.from;
                let to = response.payload.parse::<SocketAddrV4>()?;
                if options.global_args.no_color {
                    println!("\n  Created TCP Connection:");
                    println!("  • From: /node/{}", from);
                    println!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port());
                } else {
                    println!("\n  Created TCP Connection:");
                    println!("{}", format!("  • From: /node/{}", from).light_magenta());
                    println!(
                        "{}",
                        format!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port())
                            .light_magenta()
                    );
                }
            }
            OutputFormat::Json => {
                let port = options.config.get_node_port(node_name)?;
                let route: Route = route![(TCP, format!("localhost:{}", port))]
                    .modify()
                    .append_t(TCP, response.payload.to_string())
                    .into();
                let multiaddr = route_to_multiaddr(&route)
                    .context("Couldn't convert given address into `MultiAddr`")?;
                let json = json!([{"route": multiaddr.to_string() }]);
                println!("{}", json);
            }
        }
        Ok(())
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (options, command): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node_name = extract_address_value(&command.node_opts.from)?;
    let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
    let request = api::create_tcp_connection(&command);
    rpc.request(request).await?;
    let response = rpc.parse_response::<models::transport::TransportStatus>()?;

    command.print_output(&node_name, &options, &response)
}
