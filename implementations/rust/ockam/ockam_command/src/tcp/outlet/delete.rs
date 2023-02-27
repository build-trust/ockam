use crate::node::NodeOpts;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{CommandGlobalOpts};
use anyhow::ensure;
use clap::Args;
use ockam::Context;
use ockam_api::{
    error::ApiError,
    nodes::models::portal::{DeleteOutlet, OutletStatus},
    route_to_multiaddr,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_core::route;

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command()]
pub struct DeleteCommand {
    /// Name assigned to outlet that will be deleted
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp outlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }

    // pub fn check_credential(&self) -> Option<bool> {
    //     if self.check_credential {
    //         Some(true)
    //     } else if self.disable_check_credential {
    //         Some(false)
    //     } else {
    //         None
    //     }
    // }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    // let node = extract_address_value(&cmd.at/)?;
    // let mut rpc = Rpc::background(&ctx, &options, &node)?;

    // let cmd = DeleteCommand {
    //     from: extract_address_value(&cmd.from)?,
    //     ..cmd
    // };

    // rpc.request(make_api_request(cmd)?).await?;
    // let OutletStatus { worker_addr, .. } = rpc.parse_response()?;

    // let addr = route_to_multiaddr(&route![worker_addr.to_string()])
    //     .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
    // println!("{addr}");

    // TO-DO: here, a request has to be sent to API, using a DeleteOutlet structure containing node + alias
    // This request will use the `.remove()` method from the registry's BTree. (See portals.rs AND portal.rs under ockam API)
    let node = extract_address_value(&cmd.node_opts.api_node)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;
    rpc.request(make_api_request(cmd)?).await?;

    let OutletStatus { worker_addr, .. } = rpc.parse_response()?;

    let addr = route_to_multiaddr(&route![worker_addr.to_string()])
        .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
    println!("{addr}");
    Ok(())
}

/// Construct a request to delete a tcp outlet
fn make_api_request<'a>(cmd: DeleteCommand) -> crate::Result<RequestBuilder<'a, DeleteOutlet<'a>>> {
    // let alias = cmd.alias.map(|a| a.into());
    let addr = extract_address_value(&cmd.node_opts.api_node)?;
    let payload = DeleteOutlet::new(cmd.alias, addr);
    let request = Request::delete("/node/outlet").body(payload);
    Ok(request)
}

fn alias_parser(arg: &str) -> anyhow::Result<String> {
    ensure! {
        !arg.contains(':'),
        "an outlet alias must not contain ':' characters"
    }
    Ok(arg.to_string())
}
