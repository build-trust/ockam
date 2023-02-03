use crate::CommandGlobalOpts;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct SetDefaultNodeCommand {
    /// Name of the Node
    pub name: String,
}

impl SetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(&self.name, &options) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(_name: &str, _options: &CommandGlobalOpts) -> crate::Result<()> {
    // TODO: add symlink to options.state.defaults().node
    todo!()
}
