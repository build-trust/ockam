use clap::{Command, Parser};
use miette::IntoDiagnostic;
use std::process::exit;
use std::sync::Arc;

use crate::branding::BrandingCompileEnvVars;
use crate::{
    add_command_error_event, get_env_attributes, has_help_flag, has_version_flag, pager,
    replace_hyphen_with_stdin, util::exitcode, version::Version, OckamCommand,
};
use ockam_api::cli_state::{CliState, CliStateMode};
use ockam_api::logs::{
    logging_configuration, logging_enabled, Colored, ExportingConfiguration, LogFormat,
    LogLevelWithCratesFilter, LoggingTracing,
};
use ockam_api::output::Output;
use ockam_node::{Context, NodeBuilder};

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

    // allows environment variables to be set via command line as a workaround for the lack of
    // environment variable support in kubernetes probes
    let input = if let Some((attributes, remaining_arguments)) = get_env_attributes(&input)? {
        for (key, value) in attributes {
            // set the environment variable while we are still single-threaded
            // and before the arguments are parsed
            std::env::set_var(key, value);
        }
        remaining_arguments
    } else {
        input
    };

    let no_args_passed = input.len() <= 1;
    let command_parsing_res = if no_args_passed {
        Ok(OckamCommand::default())
    } else {
        OckamCommand::try_parse_from(&input)
    };

    let node_builder = NodeBuilder::new().no_logging();

    let (ctx, executor) = node_builder.build();

    executor.execute(async move {
        let res = match command_parsing_res {
            Ok(command) => command.run(&ctx, &input).await,
            Err(err) => handle_invalid_command(&input, err, &ctx).await,
        };

        ctx.shutdown_node().await?;

        res
    })??;

    Ok(())
}

async fn handle_invalid_command(
    input: &[String],
    help: clap::Error,
    ctx: &Context,
) -> miette::Result<()> {
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
        let cli_state = CliState::create(CliStateMode::InMemory).await?;
        let level_and_crates = LogLevelWithCratesFilter::new().into_diagnostic()?;
        let logging_configuration = logging_configuration(
            level_and_crates,
            None,
            Colored::On,
            LogFormat::Default,
            logging_enabled()?,
        );
        let exporting_configuration = ExportingConfiguration::foreground(&cli_state, ctx)
            .await
            .into_diagnostic()?;
        let cli_state = Arc::new(cli_state);
        let _guard = LoggingTracing::setup(
            cli_state.clone(),
            &logging_configuration.into_diagnostic()?,
            &exporting_configuration,
            "local node",
            None,
            ctx,
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

/// Return the names of the top-level commands, i.e., the commands defined in [crate::OckamSubcommand]
pub fn top_level_command_names(cmd: &Command) -> Vec<String> {
    let mut names = vec![];
    let bin_name = BrandingCompileEnvVars::bin_name();
    for subcmd in cmd.get_subcommands() {
        names.push(subcmd.get_name().replace(bin_name, ""));
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_top_level_command_names() {
        let cmd = OckamCommand::command();
        let top_level_commands = top_level_command_names(&cmd);

        let top_level_commands_sample = vec!["node", "project", "tcp-inlet"];
        for name in top_level_commands_sample {
            assert!(top_level_commands.contains(&name.to_string()));
        }

        let subcommands_sample = vec!["node create", "tcp-inlet delete"];
        for name in subcommands_sample {
            assert!(!top_level_commands.contains(&name.to_string()));
        }
    }
}
