use clap::Args;
use ockam_api::cli_state::traits::{StateItemDirTrait, StateTrait};

use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the vault
    pub name: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ShowCommand) -> crate::Result<()> {
    let name = cmd
        .name
        .unwrap_or(opts.state.vaults.default()?.name().to_string());
    let state = opts.state.vaults.get(&name)?;
    println!("Vault:");
    for line in state.to_string().lines() {
        println!("{:2}{}", "", line)
    }
    Ok(())
}
