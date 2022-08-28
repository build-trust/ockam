use anyhow::Context as _;
use clap::Args;
use rand::prelude::random;

use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::{clean_multiaddr, is_local_node};
use ockam_core::api::{Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{get_final_element, node_rpc, RpcBuilder, DEFAULT_ORCHESTRATOR_ADDRESS};
use crate::Result;
use crate::{CommandGlobalOpts, HELP_TEMPLATE};

const EXAMPLES: &str = "\
EXAMPLES

    # Create two nodes
    $ ockam node create n1
    $ ockam node create n2

    # Create a forwarder to node n2 at node n1
    $ ockam forwarder create --from forwarder_to_n2 --for /node/n2 --at /node/n1

    # Send message via the forwarder
    $ ockam message send hello --to /node/n1/service/forwarder_to_n2/service/uppercase

LEARN MORE
";

#[derive(Clone, Debug, Args)]
#[clap(help_template = const_str::replace!(HELP_TEMPLATE, "LEARN MORE", EXAMPLES))]
pub struct CreateCommand {
    /// Name of the forwarder (Optional).
    #[clap(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    pub forwarder_name: String,

    /// Node for which to create the forwarder.
    #[clap(long, name = "NODE", display_order = 900)]
    to: String,

    /// Route to the node on which to create the forwarder.
    #[clap(long, name = "ROUTE", default_value = DEFAULT_ORCHESTRATOR_ADDRESS, display_order = 900)]
    at: MultiAddr,

    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    async fn go(ctx: &mut Context, opts: &CommandGlobalOpts, cmd: CreateCommand) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let api_node = get_final_element(&cmd.to);
        let at_rust_node = is_local_node(&cmd.at).context("Argument --at is not valid")?;
        let (at, meta) = clean_multiaddr(&cmd.at, &opts.config.get_lookup()).unwrap();
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.route_to_controller,
            api_node,
        )
        .await?;
        let at = crate::project::util::clean_projects_multiaddr(at, projects_sc)?;
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(&tcp).build()?;
        let cmd = CreateCommand { at, ..cmd };
        rpc.request(req(&cmd, at_rust_node)?).await?;
        rpc.parse_and_print_response::<ForwarderInfo>()?;
        Ok(())
    }
    go(&mut ctx, &opts, cmd).await
}

/// Construct a request to create a forwarder
fn req(cmd: &CreateCommand, at_rust_node: bool) -> anyhow::Result<RequestBuilder<CreateForwarder>> {
    let alias = if at_rust_node {
        let mut name = "forward_to_".to_owned();
        name.push_str(&cmd.forwarder_name);
        name
    } else {
        cmd.forwarder_name.clone()
    };

    Ok(Request::post("/node/forwarder").body(CreateForwarder::new(
        &cmd.at,
        Some(alias),
        at_rust_node,
    )))
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}
