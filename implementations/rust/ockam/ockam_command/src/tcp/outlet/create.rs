use crate::util::{connect_to, exitcode, get_final_element};
use crate::util::{ComposableSnippet, Operation, PortalMode, Protocol};
use crate::{help, CommandGlobalOpts};
use clap::Args;
use minicbor::Decoder;
use ockam::{Context, Route};
use ockam_api::{
    error::ApiError,
    nodes::models::portal::{CreateOutlet, OutletStatus},
    nodes::NODEMANAGER_ADDR,
    route_to_multiaddr,
};
use ockam_core::api::{Request, Response, Status};
use ockam_core::route;
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

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[clap(long, display_order = 900, name = "NODE")]
    at: String,

    /// Address of the tcp outlet.
    #[clap(long, display_order = 901, name = "OUTLET_ADDRESS")]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[clap(long, display_order = 902, name = "SOCKET_ADDRESS")]
    to: SocketAddr,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let bind = cc.from.to_string();
        let peer = cc.to.to_string();
        let mode = PortalMode::Outlet;

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
        let at = &self.at.clone();
        let node = get_final_element(at);
        let port = match cfg.select_node(node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        let command = CreateCommand {
            from: String::from(get_final_element(&self.from)),
            ..self
        };

        let composite = (&command).into();
        connect_to(port, command, create_outlet);

        // Update the startup config
        let startup_cfg = match cfg.get_startup_cfg(node) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("failed to load startup configuration: {}", e);
                std::process::exit(exitcode::IOERR);
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

pub async fn create_outlet(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route = base_route.modify().append(NODEMANAGER_ADDR);
    let message = make_api_request(cmd)?;
    let response: Vec<u8> = ctx.send_and_receive(route, message).await?;

    let (response, OutletStatus { worker_addr, .. }) = parse_outlet_status(&response)?;
    let addr = route_to_multiaddr(&route![worker_addr.to_string()])
        .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;

    match response.status() {
        Some(Status::Ok) => {
            println!("{}", addr);
        }

        _ => {
            eprintln!("An unknown error occurred while creating an outlet...");
            std::process::exit(exitcode::UNAVAILABLE);
        }
    }

    Ok(())
}

/// Construct a request to create a tcp outlet
fn make_api_request(cmd: CreateCommand) -> ockam::Result<Vec<u8>> {
    let tcp_addr = &cmd.to.to_string();
    let worker_addr = cmd.from;
    let alias = (None::<String>).as_ref().map(|x| x.as_str().into());
    let payload = CreateOutlet::new(tcp_addr, worker_addr, alias);

    let mut buf = vec![];
    Request::post("/node/outlet")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Parse the returned status response
fn parse_outlet_status(response: &[u8]) -> ockam::Result<(Response, OutletStatus<'_>)> {
    let mut decoder = Decoder::new(response);
    let response = decoder.decode::<Response>()?;
    Ok((response, decoder.decode::<OutletStatus>()?))
}
