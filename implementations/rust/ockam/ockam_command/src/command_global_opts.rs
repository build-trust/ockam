use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use std::process::exit;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info};
use tracing_core::Level;

use ockam_api::logs::{
    crates_filter, logging_configuration, Colored, LoggingConfiguration, LoggingTracing,
    TracingConfiguration, TracingGuard,
};
use ockam_api::CliState;

use crate::subcommand::OckamSubcommand;
use crate::terminal::color_primary;
use crate::util::exitcode;
use crate::version::Version;
use crate::{fmt_err, fmt_log, fmt_ok, GlobalArgs, Terminal, TerminalStream};

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
        let terminal = Terminal::from(global_args);
        let logging_configuration =
            Self::make_logging_configuration(global_args, cmd, terminal.is_tty())?;
        let tracing_configuration = Self::make_tracing_configuration(global_args, cmd)?;
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
                        .write_line()
                        .unwrap();
                    exit(exitcode::OK);
                }
                terminal
                    .write_line(fmt_err!("Failed to initialize local state"))
                    .unwrap();
                terminal
                    .write_line(fmt_log!(
                        "Consider upgrading to the latest version of Ockam Command"
                    ))
                    .unwrap();
                let ockam_home = std::env::var("OCKAM_HOME").unwrap_or("~/.ockam".to_string());
                terminal
                    .write_line(fmt_log!(
                        "You can also try removing the local state using {} \
                        or deleting the directory at {}",
                        color_primary("ockam reset"),
                        color_primary(ockam_home)
                    ))
                    .unwrap();
                terminal
                    .write_line(format!("\n{:?}", miette!(err.to_string())))
                    .unwrap();
                exit(exitcode::SOFTWARE);
            }
        };
        Ok(Self {
            global_args: global_args.clone(),
            state,
            terminal,
            rt: Arc::new(Runtime::new().expect("cannot initialize the tokio runtime")),
            tracing_guard,
        })
    }

    /// Set up a logger and a tracer for the current node
    /// If the node is a background node we always enable logging, regardless of environment variables
    fn setup_logging_tracing(
        cmd: &OckamSubcommand,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &TracingConfiguration,
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
        if global_args.quiet {
            return LoggingConfiguration::off().into_diagnostic();
        };

        let log_path = cmd.log_path();
        let crates = crates_filter().into_diagnostic()?;
        if cmd.is_background_node() {
            Ok(LoggingConfiguration::background(log_path, crates).into_diagnostic()?)
        } else {
            let preferred_log_level = verbose_log_level(global_args.verbose);
            let colored = if !global_args.no_color && is_tty {
                Colored::On
            } else {
                Colored::Off
            };
            Ok(
                logging_configuration(preferred_log_level, colored, log_path, crates)
                    .into_diagnostic()?,
            )
        }
    }

    /// Create the tracing configuration, depending on the command to execute
    fn make_tracing_configuration(
        global_args: &GlobalArgs,
        cmd: &OckamSubcommand,
    ) -> miette::Result<TracingConfiguration> {
        Ok(if cmd.is_background_node() {
            TracingConfiguration::background(global_args.quiet).into_diagnostic()?
        } else {
            TracingConfiguration::foreground(global_args.quiet).into_diagnostic()?
        })
    }

    /// Log the inputs and configurations used to execute the command
    fn log_inputs(
        arguments: &[String],
        global_args: &GlobalArgs,
        cmd: &OckamSubcommand,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &TracingConfiguration,
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
            tracing_guard.shutdown();
        };
    }
}

#[cfg(test)]
impl CommandGlobalOpts {
    pub fn new_for_test(global_args: GlobalArgs, state: CliState) -> Self {
        let terminal = Terminal::new(
            global_args.quiet,
            global_args.no_color,
            global_args.no_input,
            global_args.output_format.clone(),
        );
        Self {
            global_args,
            state,
            terminal,
            rt: Arc::new(Runtime::new().expect("cannot initialize the tokio runtime")),
            tracing_guard: None,
        }
    }
}

/// Return the LevelFilter corresponding to a given verbose flag
/// -vvv set verbose to 3 (3 times 'v')
/// and this function translate the number of `v` to a log level
fn verbose_log_level(verbose: u8) -> Option<Level> {
    match verbose {
        0 => None,
        1 => Some(Level::INFO),
        2 => Some(Level::DEBUG),
        _ => Some(Level::TRACE),
    }
}
