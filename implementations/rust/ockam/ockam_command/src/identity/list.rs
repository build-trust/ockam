use crate::util::{exitcode, node_rpc};
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use ockam::Context;
use crate::identity::show::print_identity;

/// List nodes
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[arg(short, long)]
    full: bool,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let identity_states = opts.state.identities.list()?;
    if identity_states.is_empty() {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("No identities registered on this system!"),
        ));
    }
    let vault = opts.state.vaults.default()?.config.get().await?;
    for state in identity_states {
        let identity = state.config.get(&ctx, &vault).await?;
        print_identity(&identity, cmd.full, &opts.global_args.output_format).await
            .map_err(|_| crate::Error::new(
            exitcode::IOERR,
            anyhow!("The identity {} cannot be loaded using the default vault.",
                identity.identifier(),
            )))?;
    }
    Ok(())
}
