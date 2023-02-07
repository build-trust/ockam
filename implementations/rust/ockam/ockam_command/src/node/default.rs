use crate::node::default_node_name;
use crate::node::util::{check_default, set_default_node};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// Changes default node
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct DefaultCommand {
    /// Name of the node.
    #[arg(default_value_t = default_node_name())]
    node_name: String,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> crate::Result<()> {
    if check_default(&opts, &cmd.node_name) {
        println!("Already set to default node");
    } else {
        set_default_node(&opts, &cmd.node_name)?;
        println!("Set node '{}' as default", &cmd.node_name);
    }
    Ok(())
}
