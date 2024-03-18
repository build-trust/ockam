use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args as ClapArgs;
use miette::Result;
use ockam_node::Context;
use std::sync::Arc;

/// Implementations of this traits return a list of commands of a given type
/// as if they had been read from arguments coming from the command line.
#[async_trait]
pub trait CommandsParser<C>: Clone
where
    C: ClapArgs,
{
    fn parse_commands(self, overrides: &ValuesOverrides) -> Result<Vec<C>>;
}

/// List of parsed commands
/// Each command can be validated then executed
pub struct ParsedCommands {
    pub commands: Vec<Arc<dyn ParsedCommand>>,
}

impl ParsedCommands {
    /// Create a list of parsed commands
    pub fn new<C: ParsedCommand + Send + 'static>(commands: Vec<C>) -> Self {
        ParsedCommands {
            commands: commands
                .into_iter()
                .map(|c| {
                    let b: Arc<dyn ParsedCommand> = Arc::new(c);
                    b
                })
                .collect::<Vec<Arc<dyn ParsedCommand>>>(),
        }
    }

    /// Validate and run each command
    pub async fn run(self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<()> {
        for cmd in self.commands.into_iter() {
            if cmd.is_valid(ctx, opts).await? {
                cmd.run(ctx, opts).await?;
                opts.terminal.write_line("")?;
            }
        }
        Ok(())
    }
}

impl<C: Command> From<Vec<C>> for ParsedCommands {
    fn from(cmds: Vec<C>) -> ParsedCommands {
        ParsedCommands::new(cmds)
    }
}

/// This trait represents a command which can be validated then executed
#[async_trait]
pub trait ParsedCommand: Send + Sync + 'static {
    /// Returns true if the command can be executed, false otherwise.
    async fn is_valid(&self, ctx: &Context, opts: &CommandGlobalOpts) -> Result<bool>;

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
        self.clone().async_run(ctx, opts.clone()).await
    }
}

/// This type is only used to properly type an empty list of ParsedCommands
struct EmptyParsedCommand;

#[async_trait]
impl ParsedCommand for EmptyParsedCommand {
    async fn run(&self, _ctx: &Context, _opts: &CommandGlobalOpts) -> Result<()> {
        Ok(())
    }

    async fn is_valid(&self, _ctx: &Context, _opts: &CommandGlobalOpts) -> Result<bool> {
        Ok(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValuesOverrides {
    pub override_node_name: Option<String>,
}

impl ValuesOverrides {
    pub fn with_override_node_name(mut self, node_name: &str) -> Self {
        self.override_node_name = Some(node_name.to_string());
        self
    }
}
