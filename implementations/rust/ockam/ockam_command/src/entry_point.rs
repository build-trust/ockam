use std::process::exit;

use clap::Parser;
use miette::IntoDiagnostic;

use ockam_api::cli_state::CliState;
use ockam_api::fmt_log;
use ockam_api::logs::{
    logging_configuration, Colored, ExportingConfiguration, LogLevelWithCratesFilter,
    LoggingTracing,
};

use crate::{
    add_command_error_event, has_help_flag, has_version_flag, pager, replace_hyphen_with_stdin,
    util::exitcode, version::Version, ErrorReportHandler, OckamCommand,
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

    if has_version_flag(&input) {
        print_version_and_exit();
    }

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

                let level_and_crates = LogLevelWithCratesFilter::new().into_diagnostic()?;
                let logging_configuration =
                    logging_configuration(level_and_crates, None, Colored::On);
                let _guard = LoggingTracing::setup(
                    &logging_configuration.into_diagnostic()?,
                    &ExportingConfiguration::foreground().into_diagnostic()?,
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
    }
    Ok(())
}

fn print_version_and_exit() {
    let version_msg = Version::long();
    let version_msg_vec = version_msg.split('\n').collect::<Vec<_>>();
    println!("{}", fmt_log!("ockam {}", version_msg_vec[0]));
    for item in version_msg_vec.iter().skip(1) {
        println!("{}", fmt_log!("{}", item));
    }
    exit(exitcode::OK);
}
