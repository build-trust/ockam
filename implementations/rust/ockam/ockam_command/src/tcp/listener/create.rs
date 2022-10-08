use crate::util::{bind_to_port_check, extract_node_name};
use crate::{
    util::{api, connect_to, exitcode},
    CommandGlobalOpts,
};
use clap::Args;
use ockam::{Context, Route, TCP};
use ockam_api::{
    nodes::{models::transport::TransportStatus, NODEMANAGER_ADDR},
    route_to_multiaddr,
};
use ockam_core::api::Status;
use std::str::FromStr;

#[derive(Args, Clone, Debug)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TCPListenerNodeOpts,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

#[derive(Clone, Debug, Args)]
pub struct TCPListenerNodeOpts {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let node = extract_node_name(&self.node_opts.at).unwrap_or_else(|_| "".to_string());
        let port = cfg.get_node_port(&node).unwrap();

        let input_addr = match std::net::SocketAddr::from_str(&self.address) {
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

        connect_to(port, self, create_listener);
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
    Ok(())
}
