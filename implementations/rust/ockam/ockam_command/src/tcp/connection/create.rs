use crate::node::default_node_name;
use crate::{
    util::{api, extract_address_value, node_rpc, Rpc},
    CommandGlobalOpts, OutputFormat,
};
use clap::Args;
use colorful::Colorful;
use ockam_api::nodes::models;
use ockam_multiaddr::proto::{DnsAddr, Tcp, OTCP};
use ockam_multiaddr::MultiAddr;
use serde_json::json;
use std::net::SocketAddrV4;

#[derive(Clone, Debug, Args)]
pub struct TcpConnectionNodeOpts {
    /// Node that will initiate the connection
    #[arg(global = true, short, long, value_name = "NODE", default_value_t = default_node_name())]
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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }

    fn print_output(
        &self,
        node_name: &str,
        opts: &CommandGlobalOpts,
        response: &models::transport::TransportStatus,
    ) -> crate::Result<()> {
        // if output format is json, write json to stdout.
        match opts.global_args.output_format {
            OutputFormat::Plain => {
                let from = &self.node_opts.from;

                let to = response.payload.parse::<SocketAddrV4>()?;
                if opts.global_args.no_color {
                    println!("\n  Created TCP Connection:");
                    println!("  • From: /node/{from}");
                    println!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port());
                    println!("\n  Transport ID : {}", response.tid);
                } else {
                    println!("\n  Created TCP Connection:");
                    println!("{}", format!("  • From: /node/{from}").light_magenta());
                    println!(
                        "{}",
                        format!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port())
                            .light_magenta()
                    );
                    println!(
                        "{}",
                        format!("\n  Transport ID : {}", response.tid).light_blue()
                    );
                }
            }
            OutputFormat::Json => {
                let port = opts
                    .state
                    .nodes
                    .get(node_name)?
                    .setup()?
                    .default_tcp_listener()?
                    .addr
                    .port();
                let mut multiaddr = MultiAddr::default();
                multiaddr.push_back(DnsAddr::new("localhost"))?; // FIXME: test
                multiaddr.push_back(Tcp::new(port))?;
                multiaddr.push_back(OTCP::new(response.payload.to_string()))?;
                let json = json!([{"route": multiaddr.to_string() }]);
                println!("{json}");
            }
        }
        Ok(())
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (options, command): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let from = &command.node_opts.from;
    let node_name = extract_address_value(from.as_str())?;
    let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
    let request = api::create_tcp_connection(&command);
    rpc.request(request).await?;
    let response = rpc.parse_response::<models::transport::TransportStatus>()?;

    command.print_output(&node_name, &options, &response)
}
