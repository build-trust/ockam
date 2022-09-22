use crate::{
    util::{api, connect_to, exitcode, get_final_element},
    CommandGlobalOpts, OutputFormat,
};
use clap::Args;
use colorful::Colorful;
use ockam::{Context, Route, TCP};
use ockam_api::{
    config::snippet::{ComposableSnippet, Operation, Protocol, RemoteMode},
    nodes::{models::transport::TransportStatus, NODEMANAGER_ADDR},
    route_to_multiaddr,
};
use ockam_core::api::Status;
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
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TcpConnectionNodeOpts,

    /// The address to connect to (required)
    #[arg(name = "to", short, long, value_name = "ADDRESS")]
    pub address: String,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let mode = RemoteMode::Connector;
        let tcp = true;
        let address = cc.address.clone();

        Self {
            id: format!(
                "_transport_{}_{}_{}",
                mode,
                if tcp { "tcp" } else { "unknown" },
                address
            ),
            op: Operation::Transport {
                protocol: Protocol::Tcp,
                address,
                mode,
            },
            params: vec![],
        }
    }
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let node = get_final_element(&self.node_opts.from);
        let port = cfg.get_node_port(node);

        connect_to(port, (self.clone(), options.clone()), create_connection);

        let composite = (&self).into();
        let node = node.to_string();

        let startup_config = match cfg.startup_cfg(&node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }
        };
        startup_config.add_composite(composite);
        if let Err(e) = startup_config.persist_config_updates() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }
    }
}

pub async fn create_connection(
    ctx: Context,
    (cmd, opts): (CreateCommand, CommandGlobalOpts),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = match ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_tcp_connection(&cmd)?,
        )
        .await
    {
        Ok(sr_msg) => sr_msg,
        Err(e) => {
            eprintln!("Wasn't able to send or receive `Message`: {}", e);
            std::process::exit(exitcode::IOERR);
        }
    };

    let (response, TransportStatus { payload, .. }) = api::parse_transport_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            let r: Route = base_route
                .modify()
                .pop_back()
                .append_t(TCP, payload.to_string())
                .into();
            let multiaddr = match route_to_multiaddr(&r) {
                Some(addr) => addr,
                None => {
                    eprintln!("Couldn't convert given address into `MultiAddr`");
                    std::process::exit(exitcode::SOFTWARE);
                }
            };

            let from = cmd.node_opts.from;
            let to = cmd.address.parse::<SocketAddrV4>().unwrap();

            // if output format is json, write json to stdout.
            match opts.global_args.output_format {
                OutputFormat::Plain => {
                    if opts.global_args.no_color {
                        eprintln!("\n  Created TCP Connection:");
                        eprintln!("  • From: /node/{}", from);
                        eprintln!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port());
                    } else {
                        eprintln!("\n  Created TCP Connection:");
                        eprintln!("{}", format!("  • From: /node/{}", from).light_magenta());
                        eprintln!(
                            "{}",
                            format!("  •   To: {} (/ip4/{}/tcp/{})", to, to.ip(), to.port())
                                .light_magenta()
                        );
                    }
                }
                OutputFormat::Json => {
                    let json = json!([{"route": multiaddr.to_string() }]);
                    eprintln!("{}", json);
                }
            }
        }
        _ => {
            eprintln!(
                "An error occurred while creating the tcp connection: {}",
                payload
            );
            std::process::exit(exitcode::CANTCREAT);
        }
    }
    Ok(())
}
