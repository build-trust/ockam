use crate::command::{CommandResult, Run};
use crate::spinner::Spinner;
use crate::AppError;
use clap::ArgMatches;
use comfy_table::Table;
use log::error;
use std::thread;
use std::time::Duration;

pub struct InletCommand {}

impl Run for InletCommand {
    fn run(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        println!("Running Inlet command");

        if args.is_none() {
            error!("Inlet command requires some arguments");
            return Err(AppError::InvalidArgument);
        }

        let args = args.unwrap();

        let (subcommand, sub_args) = args.subcommand();

        match subcommand {
            "create" => self.create(sub_args),
            _ => Err(AppError::InvalidCommand),
        }
    }
}

impl InletCommand {
    pub fn create(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        if args.is_none() {
            error!("Create Inlet requires arguments");
            return Err(AppError::InvalidArgument);
        }

        let args = args.unwrap();

        let host = args.value_of("host");

        if host.is_none() {
            error!("Create Inlet requires a host argument.");
            return Err(AppError::InvalidArgument);
        }

        let host = host.unwrap();

        let port = args.value_of("port");

        if port.is_none() {
            error!("Create Inlet requires a port argument.");
            return Err(AppError::InvalidArgument);
        }

        let port = port.unwrap();

        let port: u16 = match port.parse() {
            Ok(port) => port,
            _ => {
                error!("Invalid port '{}'", port);
                return Err(AppError::InvalidArgument);
            }
        };

        let host_and_port = format!("{}:{}", host, port);

        println!("Create Inlet on {}", host_and_port);

        Ok(CommandResult {})
    }
}
