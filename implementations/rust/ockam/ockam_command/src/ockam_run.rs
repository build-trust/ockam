use clap::Parser;
use miette::IntoDiagnostic;

use crate::{
    add_command_error_event, has_help_flag, pager, replace_hyphen_with_stdin, OckamCommand,
};
use ockam_api::cli_state::CliState;
use ockam_api::logs::{LoggingConfiguration, LoggingTracing, TracingConfiguration};

/// Main method for running the `ockam` executable:
///
///  - Parse the input arguments
///  - Display the help if the arguments cannot be parsed and store a user journey error
///
pub fn run() -> miette::Result<()> {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();

    match OckamCommand::try_parse_from(input.clone()) {
        Err(help) => {
            // the -h or --help flag must not be interpreted as an error
            if has_help_flag(&input) {
                let command = input
                    .iter()
                    .take_while(|a| !a.starts_with('-'))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                let _guard = LoggingTracing::setup(
                    &LoggingConfiguration::foreground().into_diagnostic()?,
                    &TracingConfiguration::foreground(true).into_diagnostic()?,
                    "local node",
                );
                let cli_state = CliState::with_default_dir()?;
                let message = format!("could not parse the command: {}", command);
                add_command_error_event(cli_state, &command, &message, input.join(" "))?;
            };
            pager::render_help(help);
        }
        Ok(command) => command.run(input)?,
    };
    Ok(())
}
