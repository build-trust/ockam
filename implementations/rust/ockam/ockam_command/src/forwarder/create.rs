use anyhow::{anyhow, Context as _};
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam_multiaddr::proto::Project;

use ockam::{Context, TcpTransport};
use ockam_api::is_local_node;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_core::api::Request;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::util::output::Output;
use crate::util::{extract_address_value, node_rpc, process_nodes_multiaddr, RpcBuilder};
use crate::Result;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Forwarders
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the forwarder (optional)
    #[arg(hide_default_value = true, default_value = "default")]
    forwarder_name: String,

    /// Node for which to create the forwarder
    #[arg(long, id = "NODE", display_order = 900)]
    to: String,

    /// Route to the node at which to create the forwarder (optional)
    #[arg(long, id = "ROUTE", display_order = 900)]
    at: MultiAddr,

    /// Authorized identity for secure channel connection (optional)
    #[arg(long, id = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let api_node = extract_address_value(&cmd.to)?;
    let at_rust_node = is_local_node(&cmd.at).context("Argument --at is not valid")?;

    let ma = process_nodes_multiaddr(&cmd.at, &opts.state)?;

    let req = {
        let alias = if at_rust_node {
            format!("forward_to_{}", cmd.forwarder_name)
        } else {
            cmd.forwarder_name.clone()
        };
        let body = if cmd.at.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateForwarder::at_project(ma, Some(alias))
        } else {
            CreateForwarder::at_node(ma, Some(alias), at_rust_node, cmd.authorized)
        };
        Request::post("/node/forwarder").body(body)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &api_node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_and_print_response::<ForwarderInfo>()?;

    Ok(())
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}
