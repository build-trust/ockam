use crate::command::{CommandResult, Run};
use crate::AppError;
use clap::ArgMatches;
use log::{error, info};

pub struct OutletCommand {}

impl Run for OutletCommand {
    fn run(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        if args.is_none() {
            error!("Outlet command requires some arguments");
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

impl OutletCommand {
    pub fn create(&mut self, args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        if args.is_none() {
            error!("Create Outlet requires arguments");
            return Err(AppError::InvalidArgument);
        }

        let args = args.unwrap();

        let listen = args.value_of("listen");

        if listen.is_none() {
            error!("Create Outlet requires a host argument.");
            return Err(AppError::InvalidArgument);
        }

        let listen = listen.unwrap();

        let name = args.value_of("name");
        if name.is_none() {
            error!("Create Outlet requires a name argument");
            return Err(AppError::InvalidArgument);
        }

        let name = name.unwrap();

        let target = args.value_of("target");

        if target.is_none() {
            error!("Create Outlet requires a target argument");
            return Err(AppError::InvalidArgument);
        }

        let target = target.unwrap();

        info!(
            "Creating Outlet '{}' on {} with a destination of {}",
            name, listen, target
        );

        Ok(CommandResult {})
    }
}
