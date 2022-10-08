use crate::util::{connect_to, exitcode, extract_node_name};
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
Examples:

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
#[command(help_template = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: String,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS")]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS")]
    to: SocketAddr,

    /// Enable credentials authorization
    #[arg(long, short, display_order = 802)]
    pub check_credential: bool,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) -> anyhow::Result<()> {
        let cfg = &options.config;
        let at = &self.at.clone();
        let node = extract_node_name(at)?;
        let port = cfg.get_node_port(&node).unwrap();

        let command = CreateCommand {
            from: extract_node_name(&self.from)?,
            ..self
        };

        connect_to(port, command, create_outlet);
        Ok(())
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
    let payload = CreateOutlet::new(tcp_addr, worker_addr, alias, cmd.check_credential);

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
