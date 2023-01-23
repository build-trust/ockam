use crate::CommandGlobalOpts;
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
    opts.state.identities.delete(&cmd.name)?;
    println!("Identity '{}' deleted", cmd.name);
    Ok(())
}
