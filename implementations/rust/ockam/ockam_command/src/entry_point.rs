use clap::Parser;
use miette::IntoDiagnostic;
use tracing_core::Level;

use crate::{
    add_command_error_event, has_help_flag, pager, replace_hyphen_with_stdin, ErrorReportHandler,
    OckamCommand,
};
use ockam_api::cli_state::CliState;
use ockam_api::logs::{
    crates_filter, logging_configuration, Colored, LoggingTracing, TracingConfiguration,
};

/// Main method for running the `ockam` executable:
///
///  - Parse the input arguments
///  - Display the help if the arguments cannot be parsed and store a user journey error
///
pub fn run() -> miette::Result<()> {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();

    let _ = miette::set_hook(Box::new(|_e| Box::new(ErrorReportHandler::new())));

    match OckamCommand::try_parse_from(input.clone()) {
        Err(help) => {
            // the -h or --help flag must not be interpreted as an error
            if !has_help_flag(&input) {
                let command = input
                    .iter()
                    .take_while(|a| !a.starts_with('-'))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");

                let logging_configuration = logging_configuration(
                    Some(Level::TRACE),
                    Colored::On,
                    None,
                    crates_filter().into_diagnostic()?,
                );
                let _guard = LoggingTracing::setup(
                    &logging_configuration.into_diagnostic()?,
                    &TracingConfiguration::foreground(true).into_diagnostic()?,
                    "local node",
                    None,
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
