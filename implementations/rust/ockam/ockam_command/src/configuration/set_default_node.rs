use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use clap::Args;
use miette::IntoDiagnostic;
use ockam_node::Context;

#[derive(Clone, Debug, Args)]
pub struct SetDefaultNodeCommand {
    /// Name of the Node
    pub name: String,
}

impl SetDefaultNodeCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SetDefaultNodeCommand),
) -> miette::Result<()> {
    opts.state
        .set_default_node(&cmd.name)
        .await
        .into_diagnostic()
}
