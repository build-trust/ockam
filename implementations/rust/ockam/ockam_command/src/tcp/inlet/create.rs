use crate::util::{bind_to_port_check, connect_to, exitcode, get_final_element, stop_node};
use crate::util::{ComposableSnippet, Operation, PortalMode, Protocol};
use crate::CommandGlobalOpts;
use clap::Args;
use minicbor::Decoder;
use ockam::{Context, Route};
use ockam_api::{
    clean_multiaddr, nodes::models, nodes::models::portal::InletStatus, nodes::NODEMANAGER_ADDR,
};
use ockam_core::api::{Method, Request, Response, Status};
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[clap(long, display_order = 900, name = "NODE")]
    at: String,

    /// Address on which to accept tcp connections.
    #[clap(long, display_order = 900, name = "SOCKET_ADDRESS")]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[clap(long, display_order = 900, name = "ROUTE")]
    to: MultiAddr,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let bind = cc.from.to_string();
        let peer = cc.to.to_string();
        let mode = PortalMode::Inlet;

        Self {
            id: format!("_portal_{}_{}_{}_{}", mode, "tcp", bind, peer,),
            op: Operation::Portal {
                mode,
                protocol: Protocol::Tcp,
                bind,
                peer,
            },
            params: vec![],
        }
    }
}

impl CreateCommand {
    pub fn run(options: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = &options.config;
        let command = CreateCommand {
            to: match clean_multiaddr(&command.to, &cfg.get_lookup()) {
                Some((addr, _meta)) => addr,
                None => {
                    eprintln!("failed to normalize MultiAddr route");
                    std::process::exit(exitcode::USAGE);
                }
            },
            ..command
        };

        let node = get_final_element(&command.at);
        let port = match cfg.select_node(node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        // Check if the port is used by some other services or process
        if !bind_to_port_check(&command.from) {
            eprintln!("Another process is listening on the provided port!");
            std::process::exit(exitcode::IOERR);
        }

        let composite = (&command).into();
        let node = node.to_string();
        connect_to(port, command, create_inlet);

        // Update the startup config
        let startup_cfg = match cfg.get_startup_cfg(&node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(-1);
            }
        };

        startup_cfg.add_composite(composite);
        if let Err(e) = startup_cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        } else {
            std::process::exit(exitcode::OK);
        }
    }
}

pub async fn create_inlet(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route = base_route.modify().append(NODEMANAGER_ADDR);
    let message = make_api_request(&cmd.from.to_string(), &cmd.to, &None::<String>)?;
    let response: Vec<u8> = ctx.send_and_receive(route, message).await?;

    let (response, InletStatus { bind_addr, .. }) = parse_inlet_status(&response)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("{}", bind_addr)
        }

        _ => {
            eprintln!("An unknown error occurred while creating an inlet...");
            std::process::exit(exitcode::UNAVAILABLE)
        }
    }

    stop_node(ctx).await
}

/// Construct a request to create a tcp inlet
fn make_api_request(
    bind_addr: &str,
    outlet_route: &MultiAddr,
    alias: &Option<String>,
) -> ockam::Result<Vec<u8>> {
    let payload = models::portal::CreateInlet::new(
        bind_addr,
        outlet_route.to_string(),
        alias.as_ref().map(|x| x.as_str().into()),
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/inlet")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Parse the returned status response
fn parse_inlet_status(resp: &[u8]) -> ockam::Result<(Response, models::portal::InletStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<models::portal::InletStatus>()?))
}
