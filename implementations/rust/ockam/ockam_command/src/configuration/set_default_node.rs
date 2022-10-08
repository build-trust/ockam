use crate::{util::extract_node_name, CommandGlobalOpts};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct SetDefaultNodeCommand {
    /// Name of the Node
    pub name: String,
}

impl SetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(&self.name, &options) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(name: &str, options: &CommandGlobalOpts) -> crate::Result<()> {
    options.config.get_node(name)?;
    options.config.set_default_node(&extract_node_name(name)?);
    options.config.persist_config_updates()?;

    Ok(())
}
