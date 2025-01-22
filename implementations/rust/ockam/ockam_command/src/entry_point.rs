use std::process::exit;

use clap::Parser;
use miette::IntoDiagnostic;

use crate::{
    add_command_error_event, has_help_flag, has_version_flag, pager, replace_hyphen_with_stdin,
    util::exitcode, version::Version, OckamCommand,
};
use ockam_api::cli_state::{CliState, CliStateMode};
use ockam_api::log;
use ockam_api::logs::{
    logging_configuration, Colored, ExportingConfiguration, LogLevelWithCratesFilter,
    LoggingTracing,
};
use ockam_api::output::Output;
use ockam_node::NodeBuilder;

/// Main method for running the command executable:
///
///  - Parse the input arguments
///  - Display the help if the arguments cannot be parsed and store a user journey error
///
pub fn run() -> miette::Result<()> {
    // Setup the default rustls crypto provider, this is a required step when
    // multiple backends ring/aws-lc are pulled in directly, or indirectly.
    #[cfg(feature = "aws-lc")]
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install aws-lc crypto provider");

    #[cfg(all(feature = "rust-crypto", not(feature = "aws-lc")))]
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install ring crypto provider");

    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();

    if has_version_flag(&input) {
        print_version_and_exit();
    }

    log("Point 0.0");

    let command_res = OckamCommand::try_parse_from(&input);

    log("Point 0.1");

    let node_builder = NodeBuilder::new().no_logging();

    log("Point 0.2");

    let (mut ctx, mut executor) = node_builder.build();

    log("Point 0.3");

    executor.execute(async move {
        log("Point 0.4");

        match command_res {
            Ok(command) => command.run(&mut ctx, &input).await?,
            Err(err) => handle_invalid_command(&input, err).await?,
        }

        log("Point 0.5");

        ctx.shutdown_node().await?;

        log("Point 0.6");

        Ok::<(), miette::Error>(())
    })??;

    log("Point 0.7");

    Ok(())
}

async fn handle_invalid_command(input: &[String], help: clap::Error) -> miette::Result<()> {
    // log("Point 1.0");

    // the -h or --help flag must not be interpreted as an error
    if !has_help_flag(input) {
        let command = input
            .iter()
            .take_while(|a| !a.starts_with('-'))
            .collect::<Vec<_>>()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        // FIXME
        let cli_state = CliState::create(CliStateMode::InMemory).await?;
        let level_and_crates = LogLevelWithCratesFilter::new().into_diagnostic()?;
        let logging_configuration = logging_configuration(level_and_crates, None, Colored::On);
        let _guard = LoggingTracing::setup(
            &logging_configuration.into_diagnostic()?,
            &ExportingConfiguration::foreground(&cli_state)
                .await
                .into_diagnostic()?,
            "local node",
            None,
        );

        let message = format!("could not parse the command: {}", command);
        add_command_error_event(cli_state, &command, &message, input.join(" ")).await?;
    };
    pager::render_help(help);

    Ok(())
}

fn print_version_and_exit() {
    println!(
        "{}",
        Version::new()
            .multiline()
            .item()
            .expect("Failed to process version")
    );
    exit(exitcode::OK);
}
