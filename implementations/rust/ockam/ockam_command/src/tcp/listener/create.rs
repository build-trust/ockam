use crate::util::{bind_to_port_check, get_final_element};
use crate::{
    util::{api, connect_to, exitcode, stop_node},
    CommandGlobalOpts,
};
use clap::Args;
use ockam::{Context, Route, TCP};
use ockam_api::{
    config::snippet::{ComposableSnippet, Operation, Protocol, RemoteMode},
    nodes::{models::transport::TransportStatus, NODEMANAGER_ADDR},
    route_to_multiaddr,
};
use ockam_core::api::Status;
use std::str::FromStr;

#[derive(Args, Clone, Debug)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: TCPListenerNodeOpts,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

#[derive(Clone, Debug, Args)]
pub struct TCPListenerNodeOpts {
    /// Node at which to create the listener
    #[clap(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let mode = RemoteMode::Listener;
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
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let node = get_final_element(&command.node_opts.at);
        let port = match cfg.select_node(node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        let input_addr = match std::net::SocketAddr::from_str(&command.address) {
            Ok(value) => value,
            _ => {
                eprintln!("Invalid Input Address");
                std::process::exit(exitcode::IOERR);
            }
        };

        // Check if the port is used by some other services or process
        if !bind_to_port_check(&input_addr) {
            eprintln!("Another process is listening on the provided port!");
            std::process::exit(exitcode::IOERR);
        }

        connect_to(port, command.clone(), create_listener);

        let composite = (&command).into();
        let node = node.to_string();

        let startup_config = match cfg.get_startup_cfg(&node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }
        };
        startup_config.add_composite(composite);
        if let Err(e) = startup_config.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }
    }
}

pub async fn create_listener(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = match ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_tcp_listener(&cmd)?,
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

            println!(
                "Tcp listener created! You can send messages to it via this route:\n`{}`",
                multiaddr
            )
        }
        _ => {
            eprintln!(
                "An error occurred while creating the tcp listener: {}",
                payload
            );
            std::process::exit(exitcode::CANTCREAT);
        }
    }
    stop_node(ctx).await
}
