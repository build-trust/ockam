use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    name: String,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    let identity_state = opts.state.identities.get(&cmd.name)?;
    for node in opts.state.nodes.list()? {
        if node.config.identity_config()?.identifier == identity_state.config.identifier {
            return Err(anyhow!(
                "Can't delete identity '{}' because is currently in use by node '{}'",
                &cmd.name,
                &node.config.name
            )
            .into());
        }
    }
    opts.state.identities.delete_by_name(&cmd.name)?;
    println!("Identity '{}' deleted", cmd.name);
    Ok(())
}
