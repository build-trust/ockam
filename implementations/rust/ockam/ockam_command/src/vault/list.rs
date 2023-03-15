use anyhow::anyhow;
use clap::Args;

use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand;

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts) -> crate::Result<()> {
    let states = opts.state.vaults.list()?;
    if states.is_empty() {
        return Err(anyhow!("No vaults registered on this system!").into());
    }
    for (idx, vault) in states.iter().enumerate() {
        println!("Vault[{idx}]:");
        for line in vault.to_string().lines() {
            println!("{:2}{}", "", line)
        }
    }
    Ok(())
}
