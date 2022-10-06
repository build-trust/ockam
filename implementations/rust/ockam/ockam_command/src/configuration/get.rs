use crate::{util::exitcode, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(options: CommandGlobalOpts, cmd: GetCommand) -> crate::Result<()> {
    let lookup = options.config.lookup();
    match lookup.get_node(&cmd.alias) {
        Some(addr) => {
            println!("Node: {}\nAddress: {}", cmd.alias, addr);
            Ok(())
        }
        None => Err(crate::error::Error::new(
            exitcode::DATAERR,
            anyhow!(
                "Alias {} not known.  Add it first with `ockam alias set`!",
                cmd.alias
            ),
        )),
    }
}
