use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use std::process::exit;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info};

use ockam_api::colors::color_primary;
use ockam_api::logs::{
    logging_configuration, Colored, ExportingConfiguration, LogLevelWithCratesFilter,
    LoggingConfiguration, LoggingTracing, TracingGuard,
};
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{fmt_err, fmt_log, fmt_ok, CliState};

use crate::subcommand::OckamSubcommand;
use crate::util::exitcode;
use crate::version::Version;
use crate::GlobalArgs;

/// This struct contains the main structs used to implement commands:
///
///  - The arguments applicable to all commands
///  - The CliState, which provides an access to both the local state and interfaces to remote nodes
///  - The terminal used to output the command results
///
#[derive(Clone, Debug)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub state: CliState,
    pub terminal: Terminal<TerminalStream<Term>>,
    pub rt: Arc<Runtime>,
    tracing_guard: Option<Arc<TracingGuard>>,
}

impl CommandGlobalOpts {
    /// Create new CommandGlobalOpts:
    ///
    ///  - Instantiate logging + tracing
    ///  - Initialize the CliState
    ///  - Get the runtime
    ///
    pub fn new(
        arguments: &[String],
        global_args: &GlobalArgs,
        cmd: &OckamSubcommand,
    ) -> miette::Result<Self> {
        let rt = Arc::new(Runtime::new().expect("cannot initialize the tokio runtime"));
        let logging_configuration =
            Self::make_logging_configuration(global_args, cmd, Term::stdout().is_term())?;
        let tracing_configuration = Self::make_tracing_configuration(cmd)?;
        let terminal = Terminal::new(
            logging_configuration.is_enabled(),
            logging_configuration.log_dir().is_some(),
            global_args.quiet,
            global_args.no_color,
            global_args.no_input,
            global_args.output_format(),
        );
        let tracing_guard =
            Self::setup_logging_tracing(cmd, &logging_configuration, &tracing_configuration);

        Self::log_inputs(
            arguments,
            global_args,
            cmd,
            &logging_configuration,
            &tracing_configuration,
        );

        let state = match CliState::with_default_dir() {
            Ok(state) => state.set_tracing_enabled(tracing_configuration.is_enabled()),
            Err(err) => {
                // If the user is trying to run `ockam reset` and the local state is corrupted,
                // we can try to hard reset the local state.
                if let OckamSubcommand::Reset(c) = cmd {
                    c.hard_reset();
                    terminal
                        .stdout()
                        .plain(fmt_ok!("Local Ockam configuration deleted"))
                        .write_line()?;
                    exit(exitcode::OK);
                }
                terminal.write_line(fmt_err!("Failed to initialize local state"))?;
                terminal.write_line(fmt_log!(
                    "Consider upgrading to the latest version of Ockam Command"
                ))?;
                let ockam_home = std::env::var("OCKAM_HOME").unwrap_or("~/.ockam".to_string());
                terminal.write_line(fmt_log!(
                    "You can also try removing the local state using {} \
                        or deleting the directory at {}",
                    color_primary("ockam reset"),
                    color_primary(ockam_home)
                ))?;
                terminal.write_line(format!("\n{:?}", miette!(err.to_string())))?;
                exit(exitcode::SOFTWARE);
            }
        };
        Ok(Self {
            global_args: global_args.clone(),
            state,
            terminal,
            rt,
            tracing_guard,
        })
    }

    /// Set up a logger and a tracer for the current node
    /// If the node is a background node we always enable logging, regardless of environment variables
    fn setup_logging_tracing(
        cmd: &OckamSubcommand,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &ExportingConfiguration,
    ) -> Option<Arc<TracingGuard>> {
        if !logging_configuration.is_enabled() && !tracing_configuration.is_enabled() {
            return None;
        };

        let app_name = if cmd.is_background_node() {
            "local node"
        } else {
            "cli"
        };
        let tracing_guard = LoggingTracing::setup(
            logging_configuration,
            tracing_configuration,
            app_name,
            cmd.node_name(),
        );
        Some(Arc::new(tracing_guard))
    }

    /// Create the logging configuration, depending on the command to execute
    fn make_logging_configuration(
        global_args: &GlobalArgs,
        cmd: &OckamSubcommand,
        is_tty: bool,
    ) -> miette::Result<LoggingConfiguration> {
        let log_path = cmd.log_path();
        if cmd.is_background_node() {
            Ok(LoggingConfiguration::background(log_path).into_diagnostic()?)
        } else {
            let level_and_crates =
                LogLevelWithCratesFilter::from_verbose(global_args.verbose).into_diagnostic()?;
            let log_path = if level_and_crates.explicit_verbose_flag || cmd.is_foreground_node() {
                None
            } else {
                Some(CliState::command_log_path(cmd.name().as_str())?)
            };
            let colored = if !global_args.no_color && is_tty && log_path.is_none() {
                Colored::On
            } else {
                Colored::Off
            };
            Ok(logging_configuration(level_and_crates, log_path, colored).into_diagnostic()?)
        }
    }

    /// Create the tracing configuration, depending on the command to execute
    fn make_tracing_configuration(cmd: &OckamSubcommand) -> miette::Result<ExportingConfiguration> {
        Ok(if cmd.is_background_node() {
            ExportingConfiguration::background().into_diagnostic()?
        } else {
            ExportingConfiguration::foreground().into_diagnostic()?
        })
    }

    /// Log the inputs and configurations used to execute the command
    fn log_inputs(
        arguments: &[String],
        global_args: &GlobalArgs,
        cmd: &OckamSubcommand,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &ExportingConfiguration,
    ) {
        debug!("Arguments: {}", arguments.join(" "));
        debug!("Global arguments: {:#?}", &global_args);
        debug!("Command: {:#?}", &cmd);
        debug!("Version: {}", Version::short());

        info!("Tracing initialized");
        debug!("{:#?}", logging_configuration);
        debug!("{:#?}", tracing_configuration);
    }

    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.global_args = clone.global_args.set_quiet();
        clone.terminal = clone.terminal.set_quiet();
        clone
    }

    /// Flush spans and log records
    pub fn force_flush(&self) {
        if let Some(tracing_guard) = self.tracing_guard.clone() {
            tracing_guard.force_flush();
        };
    }

    /// Shutdown resources
    pub fn shutdown(&self) {
        if let Some(tracing_guard) = self.tracing_guard.clone() {
            tracing_guard.force_flush();
            tracing_guard.shutdown();
        };
    }
}
