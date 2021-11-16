use crate::command::node::NodeCommand;
use crate::AppError;
use clap::ArgMatches;
use std::str::FromStr;

pub mod node;

pub struct CommandResult {}

pub trait Run {
    fn run(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError>;
}

pub struct Command(pub Box<dyn Run>);

impl Run for Command {
    fn run(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        self.0.run(args)
    }
}

impl FromStr for Command {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "node" => Ok(Command(Box::new(NodeCommand {}))),
            _ => Err(AppError::InvalidCommand),
        }
    }
}
