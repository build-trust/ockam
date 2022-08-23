use clap::Args;

use crate::CommandGlobalOpts;
use ockam::{Context, TcpTransport};
use ockam_api::clean_multiaddr;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_core::api::{Method, Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{
    get_final_element, node_rpc, stop_node, RpcBuilder, DEFAULT_ORCHESTRATOR_ADDRESS,
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node for which to create the forwarder.
    #[clap(long = "for", name = "NODE", display_order = 900)]
    for_node: String,

    /// Route to the node on which to create the forwarder.
    #[clap(long, name = "ROUTE", default_value = DEFAULT_ORCHESTRATOR_ADDRESS, display_order = 900)]
    at: MultiAddr,

    /// Forwarding address.
    #[clap(long = "from", display_order = 900)]
    from: Option<String>,

    /// Forwarding address.
    #[clap(hide = true)]
    address: Option<String>,

    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    async fn go(
        ctx: &mut Context,
        opts: &CommandGlobalOpts,
        cmd: CreateCommand,
    ) -> crate::Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let api_node = get_final_element(&cmd.for_node);
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
        rpc.request(req(&cmd)).await?;
        rpc.print_response::<ForwarderInfo>()?;
        Ok(())
    }
    let result = go(&mut ctx, &opts, cmd).await;
    stop_node(ctx).await?;
    result
}

/// Construct a request to create a forwarder
fn req(cmd: &CreateCommand) -> RequestBuilder<CreateForwarder> {
    let (at_rust_node, address) = match &cmd.from {
        Some(s) => (true, Some(get_final_element(s))),
        None => (false, cmd.address.as_deref()),
    };
    Request::builder(Method::Post, "/node/forwarder").body(CreateForwarder::new(
        &cmd.at,
        address,
        at_rust_node,
    ))
}

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(format!("/service/{}", self.remote_address()))
    }
}
