use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{help, CommandGlobalOpts};
use anyhow::ensure;
use clap::Args;
use ockam::Context;
use ockam_api::{
    error::ApiError,
    nodes::models::portal::{CreateOutlet, OutletStatus},
    route_to_multiaddr,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_core::route;
use std::net::SocketAddr;

const HELP_DETAIL: &str = include_str!("../../constants/tcp/outlet/help_detail.txt");

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
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

    /// Enable credential authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "disable_check_credential")]
    check_credential: bool,

    /// Disable credential authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "check_credential")]
    disable_check_credential: bool,

    /// Assign a name to this outlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
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

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;

    let cmd = CreateCommand {
        from: extract_address_value(&cmd.from)?,
        ..cmd
    };

    rpc.request(make_api_request(cmd)?).await?;
    let OutletStatus { worker_addr, .. } = rpc.parse_response()?;

    let addr = route_to_multiaddr(&route![worker_addr.to_string()])
        .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
    println!("{addr}");

    Ok(())
}

/// Construct a request to create a tcp outlet
fn make_api_request<'a>(cmd: CreateCommand) -> crate::Result<RequestBuilder<'a, CreateOutlet<'a>>> {
    let tcp_addr = cmd.to.to_string();
    let check_credential = cmd.check_credential();
    let worker_addr = cmd.from;
    let alias = cmd.alias.map(|a| a.into());
    let payload = CreateOutlet::new(tcp_addr, worker_addr, alias, check_credential);
    let request = Request::post("/node/outlet").body(payload);
    Ok(request)
}

fn alias_parser(arg: &str) -> anyhow::Result<String> {
    ensure! {
        !arg.contains(':'),
        "an outlet alias must not contain ':' characters"
    }
    Ok(arg.to_string())
}
