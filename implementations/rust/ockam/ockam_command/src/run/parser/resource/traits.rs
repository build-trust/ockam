use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args as ClapArgs;
use miette::Result;
use ockam_node::Context;

#[async_trait]
pub trait ConfigRunner<C>: Clone
where
    C: ClapArgs + Command,
{
    fn len(&self) -> usize;

    async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        hooks: &PreRunHooks,
        cmds: Vec<C>,
    ) -> Result<()> {
        for mut cmd in cmds {
            if Self::pre_run_hooks(ctx, &opts, hooks, &mut cmd).await? {
                cmd.async_run(ctx, opts.clone()).await?;
                opts.terminal.write_line("")?;
            }
        }
        Ok(())
    }

    fn as_commands(&self) -> Result<Vec<C>> {
        self.clone().into_commands()
    }

    fn into_commands(self) -> Result<Vec<C>>;

    fn get_subcommand(args: &[String]) -> Result<C>;

    /// Pre-run hook that is executed right before running a command.
    ///
    /// Returns true if the command should be executed, false otherwise.
    async fn pre_run_hooks(
        _ctx: &Context,
        _opts: &CommandGlobalOpts,
        _hooks: &PreRunHooks,
        _cmd: &mut C,
    ) -> Result<bool> {
        Ok(true)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreRunHooks {
    pub override_node_name: Option<String>,
}

impl PreRunHooks {
    pub fn with_override_node_name(mut self, node_name: &str) -> Self {
        self.override_node_name = Some(node_name.to_string());
        self
    }
}
