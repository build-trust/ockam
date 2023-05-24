use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::is_tty;
use crate::{
    util::{api, extract_address_value, node_rpc, Rpc},
    CommandGlobalOpts, OutputFormat,
};
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models;
use ockam_multiaddr::proto::{DnsAddr, Tcp, Worker};
use ockam_multiaddr::MultiAddr;
use serde_json::json;

#[derive(Clone, Debug, Args)]
pub struct TcpConnectionNodeOpts {
    /// Node that will initiate the connection
    #[arg(global = true, short, long, value_name = "NODE")]
    pub from: Option<String>,
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
        initialize_node_if_default(&opts, &self.node_opts.from);
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
                if !is_tty(std::io::stdout()) {
                    let worker_addr = response
                        .worker_addr
                        .strip_prefix("0#")
                        .unwrap_or(response.worker_addr.as_ref());
                    println!("/service/{worker_addr}");
                    return Ok(());
                }
                let from = get_node_name(&opts.state, &self.node_opts.from);
                let to = response.socket_addr()?;
                if opts.global_args.no_color {
                    println!("\n  TCP Connection:");
                    println!("    From: /node/{from}");
                    println!("    To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port());
                } else {
                    println!("\n  TCP Connection:");
                    println!("{}", format!("    From: /node/{from}").light_magenta());
                    println!(
                        "{}",
                        format!("    To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port())
                            .light_magenta()
                    );
                }
            }
            OutputFormat::Json => {
                let port = opts
                    .state
                    .nodes
                    .get(node_name)?
                    .config()
                    .setup()
                    .default_tcp_listener()?
                    .addr
                    .port();
                let mut multiaddr = MultiAddr::default();
                multiaddr.push_back(DnsAddr::new("localhost"))?;
                multiaddr.push_back(Tcp::new(port))?;
                multiaddr.push_back(Worker::new(response.worker_addr.to_string()))?;
                let json = json!([{"route": multiaddr.to_string() }]);
                println!("{json}");
            }
        }
        Ok(())
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let from = get_node_name(&opts.state, &cmd.node_opts.from);
    let node_name = extract_address_value(&from)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let request = api::create_tcp_connection(&cmd);
    rpc.request(request).await?;
    let response = rpc.parse_response::<models::transport::TransportStatus>()?;

    cmd.print_output(&node_name, &opts, &response)
}
