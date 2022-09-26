use anyhow::{anyhow, Context as _};
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam_multiaddr::proto::Project;
use rand::prelude::random;

use ockam::{Context, TcpTransport};
use ockam_api::is_local_node;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_core::api::Request;
use ockam_multiaddr::{proto::Node, MultiAddr, Protocol};

use crate::forwarder::HELP_DETAIL;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{get_final_element, node_rpc, RpcBuilder};
use crate::Result;
use crate::{help, CommandGlobalOpts};

/// Create Forwarders
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    help_template = help::template(HELP_DETAIL)
)]
pub struct CreateCommand {
    /// Name of the forwarder (optional)
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    forwarder_name: String,

    /// Node for which to create the forwarder
    #[arg(long, id = "NODE", display_order = 900)]
    to: String,

    /// Route to the node at which to create the forwarder (optional)
    #[arg(long, id = "ROUTE", display_order = 900)]
    at: MultiAddr,

    /// Authorized identity for secure channel connection (optional)
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let api_node = get_final_element(&cmd.to);
    let at_rust_node = is_local_node(&cmd.at).context("Argument --at is not valid")?;

    let lookup = opts.config.lookup();

    let mut ma = MultiAddr::default();

    for proto in cmd.at.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| anyhow!("invalid node address protocol"))?;
                let addr = lookup
                    .node_address(&alias)
                    .ok_or_else(|| anyhow!("no address for node {}", &*alias))?;
                ma.try_extend(&addr)?
            }
            Project::CODE => {
                let name = proto
                    .cast::<Project>()
                    .ok_or_else(|| anyhow!("invalid project address protocol"))?;
                let proj = lookup
                    .get_project(&name)
                    .ok_or_else(|| anyhow!("no project found with name {}", &*name))?;
                ma.push_back(Project::new(&proj.id))?
            }
            _ => ma.push_back_value(&proto)?,
        }
    }

    let req = {
        let alias = if at_rust_node {
            format!("forward_to_{}", cmd.forwarder_name)
        } else {
            cmd.forwarder_name.clone()
        };
        let body = if Some(Project::CODE) == cmd.at.first().map(|p| p.code()) {
            CreateForwarder::at_project(ma, Some(alias), cmd.cloud_opts.route())
        } else {
            CreateForwarder::at_node(ma, Some(alias), at_rust_node, cmd.authorized)
        };
        Request::post("/node/forwarder").body(body)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, api_node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_and_print_response::<ForwarderInfo>()?;

    Ok(())
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}
