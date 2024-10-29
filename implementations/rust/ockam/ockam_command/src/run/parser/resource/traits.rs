use std::collections::BTreeMap;
use std::process::Stdio;

use crate::run::parser::building_blocks::{as_command_args, ArgKey, ArgValue};
use crate::run::parser::resource::utils::{binary_path, subprocess_stdio};
use crate::{Command, CommandGlobalOpts, GlobalArgs};
use async_trait::async_trait;
use miette::{IntoDiagnostic, Result};
use ockam_api::colors::color_primary;
use ockam_core::AsyncTryClone;
use ockam_node::Context;
use tokio::process::{Child, Command as ProcessCommand};
use tracing::debug;

/// This trait defines the methods that a resource must implement before it's parsed into a Command.
///
/// The resource is the layer between the configuration data and the parsed command.
pub trait Resource<C: ParsedCommand>: Sized + Send + Sync + 'static {
    const COMMAND_NAME: &'static str;

    fn args(self) -> Vec<String> {
        vec![]
    }

    fn run_in_subprocess(
        self,
        global_args: &GlobalArgs,
        extra_args: BTreeMap<ArgKey, ArgValue>,
    ) -> Result<Child> {
        let mut args = self.args();
        args.append(as_command_args(extra_args).as_mut());
        if global_args.quiet {
            args.push("--quiet".to_string());
        }
        if global_args.no_color {
            args.push("--no-color".to_string());
        }
        if global_args.no_input {
            args.push("--no-input".to_string());
        }
        if global_args.verbose > 0 {
            args.push(format!("-{}", "v".repeat(global_args.verbose as usize)));
        }
        if let Some(o) = &global_args.output_format {
            args.push("--output".to_string());
            args.push(o.to_string());
        }
        let args = Self::COMMAND_NAME
            .split(' ')
            .chain(args.iter().map(|s| s.as_str()));
        let handle = ProcessCommand::new(binary_path())
            .args(args)
            .stdout(subprocess_stdio(global_args.quiet))
            .stderr(subprocess_stdio(global_args.quiet))
            .stdin(Stdio::null())
            .spawn()
            .into_diagnostic()?;
        Ok(handle)
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
        debug!("Running command {}\n{:?}", color_primary(self.name()), self);
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
        for cmd in self.commands.into_iter() {
            if cmd.is_valid(ctx, opts).await? {
                let ctx = ctx.async_try_clone().await.into_diagnostic()?;
                cmd.run(&ctx, opts).await?;
                // Newline between commands
                opts.terminal.write_line("")?;
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
