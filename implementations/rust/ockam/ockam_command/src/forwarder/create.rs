use anyhow::Context as _;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::{clean_multiaddr, is_local_node};
use ockam_core::api::{Method, Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{
    get_final_element, node_rpc, stop_node, RpcBuilder, DEFAULT_ORCHESTRATOR_ADDRESS,
};
use crate::CommandGlobalOpts;
use crate::Result;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node for which to create the forwarder.
    #[clap(long, name = "NODE", display_order = 900)]
    to: String,

    /// Route to the node on which to create the forwarder.
    #[clap(long, name = "ROUTE", default_value = DEFAULT_ORCHESTRATOR_ADDRESS, display_order = 900)]
    at: MultiAddr,

    /// Forwarding address.
    #[clap(long, display_order = 900)]
    from: Option<String>,

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
        let projects_sc = crate::project::util::lookup_projects(
            ctx,
            opts,
            &tcp,
            &meta,
            &cmd.cloud_opts.addr,
            api_node,
        )
        .await?;
        let at = crate::project::util::clean_projects_multiaddr(at, projects_sc)?;
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(&tcp).build()?;
        let cmd = CreateCommand { at, ..cmd };
        rpc.request(req(&cmd, at_rust_node)?).await?;
        rpc.print_response::<ForwarderInfo>()?;
        Ok(())
    }
    let result = go(&mut ctx, &opts, cmd).await;
    stop_node(ctx).await?;
    result
}

/// Construct a request to create a forwarder
fn req(cmd: &CreateCommand, at_rust_node: bool) -> anyhow::Result<RequestBuilder<CreateForwarder>> {
    let alias = cmd.from.as_ref().map(|s| get_final_element(s));
    Ok(
        Request::builder(Method::Post, "/node/forwarder").body(CreateForwarder::new(
            &cmd.at,
            alias,
            at_rust_node,
        )),
    )
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}
