use clap::Args;

use crate::util::async_cmd;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "list configurations".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        for node in opts.state.get_nodes().await? {
            opts.terminal.write(format!("Node: {}\n", node.name()))?;
        }
        Ok(())
    }
}
