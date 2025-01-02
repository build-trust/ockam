use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use miette::Result;
use ockam_node::Context;
use tracing::debug;

/// This trait defines the methods that a resource must implement before it's parsed into a Command.
///
/// The resource is the layer between the configuration data and the parsed command.
pub trait Resource<C: ParsedCommand>: Sized + Send + Sync + 'static {
    const COMMAND_NAME: &'static str;

    fn args(self) -> Vec<String> {
        vec![]
    }
}

/// This trait represents a Clap command which can be validated and executed
#[async_trait]
pub trait ParsedCommand: Send + Sync + 'static {
    /// Returns true if the command can be executed, false otherwise.
    async fn is_valid(&self, _ctx: &Context, _opts: &CommandGlobalOpts) -> Result<bool> {
        Ok(true)
    }

    /// Execute the command
    async fn run(&self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<()>;
}

/// The default implementation for a ParsedCommand is a clap Command, for
/// which the validation is generally true, except in the case of an Enroll command
/// where we do some additional validation before running the command.
#[async_trait]
impl<C> ParsedCommand for C
where
    C: Command + Clone + Send + Sync + 'static,
{
    async fn is_valid(&self, _ctx: &Context, _opts: &CommandGlobalOpts) -> Result<bool> {
        Ok(true)
    }

    async fn run(&self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<()> {
        debug!("running command {} {:?}", self.name(), self);
        Ok(self.clone().async_run_with_retry(ctx, opts.clone()).await?)
    }
}

/// List of parsed commands
/// Each command can be validated then executed
pub struct ParsedCommands {
    pub commands: Vec<Box<dyn ParsedCommand>>,
}

impl ParsedCommands {
    /// Create a list of parsed commands
    pub fn new<C: ParsedCommand + Send + 'static>(commands: Vec<C>) -> Self {
        ParsedCommands {
            commands: commands
                .into_iter()
                .map(|c| {
                    let b: Box<dyn ParsedCommand> = Box::new(c);
                    b
                })
                .collect::<Vec<Box<dyn ParsedCommand>>>(),
        }
    }

    /// Validate and run each command
    pub async fn run(self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<()> {
        let len = self.commands.len();
        for (idx, cmd) in self.commands.into_iter().enumerate() {
            if cmd.is_valid(ctx, opts).await? {
                cmd.run(ctx, opts).await?;
                if idx < len - 1 {
                    opts.terminal.write_line("")?;
                }
            }
        }
        Ok(())
    }
}

impl<C: ParsedCommand> From<Vec<C>> for ParsedCommands {
    fn from(cmds: Vec<C>) -> ParsedCommands {
        ParsedCommands::new(cmds)
    }
}
