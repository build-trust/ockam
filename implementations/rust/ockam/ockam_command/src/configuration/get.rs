use clap::Args;

use crate::util::async_cmd;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "get configuration".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node_info = opts.state.get_node(&self.alias).await?;
        let addr = &node_info
            .tcp_listener_address()
            .map(|a| a.to_string())
            .unwrap_or("N/A".to_string());
        println!("Address: {addr}");
        Ok(())
    }
}
