use crate::util::{exitcode, node_rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use anyhow::anyhow;
use ockam::Context;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    name: String,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let identity_state = opts.state.identities.get(&cmd.name)?;
    for node in opts.state.nodes.list()? {
        let node_identity = node.config.identity(&ctx).await?;
        if node_identity.identifier() == &identity_state.config.identifier {
            return Err(crate::Error::new(
                exitcode::USAGE,
                anyhow!("Cannot delete identity that is being used"),
            ));
        }
    }
    opts.state.identities.delete(&cmd.name)?;
    println!("Identity deleted: {}", identity_state.config.identifier);
    Ok(())
}
