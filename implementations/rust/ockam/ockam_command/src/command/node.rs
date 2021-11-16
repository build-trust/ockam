use crate::command::{CommandResult, Run};
use crate::spinner::Spinner;
use crate::AppError;
use clap::ArgMatches;
use comfy_table::Table;
use std::thread;
use std::time::Duration;

pub struct NodeCommand {}

impl Run for NodeCommand {
    fn run(&mut self, _args: Option<&ArgMatches>) -> Result<CommandResult, AppError> {
        bunt::println!("Running Node command {$red}foobar{/$}");

        let spinner = Spinner::default();

        thread::sleep(Duration::from_secs(3));
        spinner.stop("Done");

        let mut table = Table::new();
        table
            .set_header(vec!["Node", "Host and Port", "Route"])
            .add_row(vec![
                "your_node",
                "1.hub.ockam.network:12345",
                "abcdef1234567890",
            ])
            .add_row(vec![
                "another_node",
                "2.hub.ockam.network:67890",
                "c0ffeecafe001223",
            ]);

        println!("{}", table);

        Ok(CommandResult {})
    }
}
