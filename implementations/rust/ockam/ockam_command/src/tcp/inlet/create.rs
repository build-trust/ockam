use crate::util::{bind_to_port_check, connect_to, exitcode, get_final_element};
use crate::util::{ComposableSnippet, Operation, PortalMode, Protocol};
use crate::{help, CommandGlobalOpts};
use clap::Args;
use minicbor::Decoder;
use ockam::{Context, Route};
use ockam_api::{
    clean_multiaddr, nodes::models, nodes::models::portal::InletStatus, nodes::NODEMANAGER_ADDR,
};
use ockam_core::api::{Request, Response, Status};
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;

const HELP_DETAIL: &str = "\
EXAMPLES:

```sh
    # Create a target service, we'll use a simple http server for this example
    $ python3 -m http.server --bind 127.0.0.1 5000

    # Create two nodes
    $ ockam node create n1
    $ ockam node create n2

    # Create a TCP outlet from n1 to the target server
    $ ockam tcp-outlet create --at /node/n1 --from /service/outlet --to 127.0.0.1:5000

    # Create a TCP inlet from n2 to the outlet on n1
    $ ockam tcp-inlet create --at /node/n2 --from 127.0.0.1:6000 --to /node/n1/service/outlet

    # Access the service via the inlet/outlet pair
    $ curl 127.0.0.1:6000
```
";

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: String,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS")]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE")]
    to: MultiAddr,

    /// Enable credentials authorization
    #[arg(long, short, display_order = 802)]
    pub check_credential: bool,
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
    pub fn run(self, options: CommandGlobalOpts) -> anyhow::Result<()> {
        let cfg = &options.config;
        let command = CreateCommand {
            to: match clean_multiaddr(&self.to, &cfg.lookup()) {
                Some((addr, _meta)) => addr,
                None => {
                    eprintln!("failed to normalize MultiAddr route");
                    std::process::exit(exitcode::USAGE);
                }
            },
            ..self
        };

        let node = get_final_element(&command.at);
        let port = cfg.get_node_port(node);

        // Check if the port is used by some other services or process
        if !bind_to_port_check(&command.from) {
            eprintln!("Another process is listening on the provided port!");
            std::process::exit(exitcode::IOERR);
        }

        let composite = (&command).into();
        let node = node.to_string();
        connect_to(port, command, create_inlet);

        // Update the startup config
        let startup_cfg = match cfg.startup_cfg(&node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(-1);
            }
        };

        startup_cfg.add_composite(composite);
        if let Err(e) = startup_cfg.persist_config_updates() {
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
    let message = make_api_request(
        &cmd.from.to_string(),
        &cmd.to,
        &None::<String>,
        cmd.check_credential,
    )?;
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

    Ok(())
}

/// Construct a request to create a tcp inlet
fn make_api_request(
    bind_addr: &str,
    outlet_route: &MultiAddr,
    alias: &Option<String>,
    check_credential: bool,
) -> ockam::Result<Vec<u8>> {
    let payload = models::portal::CreateInlet::new(
        bind_addr,
        outlet_route.to_string(),
        alias.as_ref().map(|x| x.as_str().into()),
        check_credential,
    );

    let mut buf = vec![];
    Request::post("/node/inlet")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Parse the returned status response
fn parse_inlet_status(resp: &[u8]) -> ockam::Result<(Response, InletStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<InletStatus>()?))
}
