use clap::Args;

use ockam_api::cloud::project::Project;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, stop_node, RpcAlt, RpcCaller};
use crate::CommandGlobalOpts;
use ockam_api::cloud::BareCloudRequestWrapper;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    /// TODO: this is not used at all?
    #[clap(display_order = 1001)]
    pub space_id: String,

    /// Id of the project.
    #[clap(display_order = 1002)]
    pub project_id: String,
    // TODO: add project_name arg that conflicts with project_id
    //  so we can call the get_project_by_name api method
    // /// Name of the project.
    // #[clap(display_order = 1002)]
    // pub project_name: String,
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    #[clap(skip)]
    pub global_opts: Option<CommandGlobalOpts>,
}

impl ShowCommand {
    pub fn run(mut self, opts: CommandGlobalOpts) {
        self.global_opts = Some(opts.clone());
        node_rpc(rpc, (opts, self));
    }
}

impl<'a> RpcCaller<'a> for ShowCommand {
    type Req = BareCloudRequestWrapper<'a>;
    type Resp = Project<'a>;

    fn req(&'a self) -> ockam_core::api::RequestBuilder<'_, Self::Req> {
        api::project::show(self)
    }
}

async fn rpc(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let res = rpc_callback(cmd, &ctx, opts).await;
    stop_node(ctx).await?;
    res
}

async fn rpc_callback(
    mut cmd: ShowCommand,
    ctx: &ockam::Context,
    opts: CommandGlobalOpts,
) -> crate::Result<()> {
    // We apply the inverse transformation done in the `create` command.
    use crate::util::output::Output;

    let node = cmd.node_opts.api_node.clone();
    RpcAlt::new(ctx, &opts, &node)?
        .request_then_response(&mut cmd)
        .await?
        .parse_body()?
        .print(&opts)?;
    Ok(())
}
