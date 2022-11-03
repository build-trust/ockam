use crate::util::{bind_to_port_check, exitcode, extract_address_value, node_rpc, RpcBuilder};
use crate::Result;
use crate::{help, CommandGlobalOpts};
use anyhow::anyhow;
use anyhow::ensure;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::Request;
use ockam_multiaddr::proto::{Node, Project};
use ockam_multiaddr::{MultiAddr, Protocol as _};
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

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
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

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Enable credentials authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "disable_check_credential")]
    check_credential: bool,

    /// Disable credentials authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "check_credential")]
    disable_check_credential: bool,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    pub fn check_credential(&self) -> Option<bool> {
        if self.check_credential {
            Some(true)
        } else if self.disable_check_credential {
            Some(false)
        } else {
            None
        }
    }
}

async fn rpc(ctx: Context, (opts, mut cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let lookup = opts.config.lookup();
    cmd.to = {
        let mut to = MultiAddr::default();
        for proto in cmd.to.iter() {
            match proto.code() {
                Node::CODE => {
                    let alias = proto
                        .cast::<Node>()
                        .ok_or_else(|| anyhow!("invalid node address protocol"))?;
                    let addr = lookup
                        .node_address(&alias)
                        .ok_or_else(|| anyhow!("no address for node {}", &*alias))?;
                    to.try_extend(&addr)?
                }
                _ => to.push_back_value(&proto)?,
            }
        }
        to
    };

    // Check if the port is used by some other services or process
    if !bind_to_port_check(&cmd.from) {
        return Err(crate::error::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let tcp = TcpTransport::create(&ctx).await?;
    let node = extract_address_value(&cmd.at)?;

    let req = {
        let check_credential = cmd.check_credential();
        let mut payload = if cmd.to.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateInlet::via_project(cmd.from, cmd.to, check_credential)
        } else {
            CreateInlet::to_node(cmd.from, cmd.to, check_credential, cmd.authorized)
        };
        if let Some(a) = cmd.alias {
            payload.set_alias(a)
        }
        Request::post("/node/inlet").body(payload)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_response::<InletStatus>()?;

    Ok(())
}

fn alias_parser(arg: &str) -> anyhow::Result<String> {
    ensure! {
        !arg.contains(':'),
        "an inlet alias must not contain ':' characters"
    }
    Ok(arg.to_string())
}
