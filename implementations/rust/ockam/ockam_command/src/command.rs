use crate::branding::{load_compile_time_vars, BrandingCompileEnvVars};
use crate::command_events::add_command_event;
use crate::command_global_opts::CommandGlobalOpts;
use crate::global_args::GlobalArgs;
use crate::subcommand::OckamSubcommand;
use crate::upgrade::check_if_an_upgrade_is_available;
use crate::version::Version;
use crate::{add_command_error_event, docs, ErrorReportHandler};
use clap::Parser;
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use ockam_api::colors::color_primary;
use ockam_api::logs::{
    is_exporting_set, logging_configuration, Colored, ExportingConfiguration,
    LogLevelWithCratesFilter, LoggingConfiguration, LoggingTracing, TracingGuard,
};
use ockam_api::terminal::Terminal;
use ockam_api::{fmt_err, fmt_log, fmt_ok, fmt_warn, CliState};
use ockam_core::OCKAM_TRACER_NAME;
use ockam_node::Context;
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Link, SpanBuilder, TraceContextExt, Tracer};
use opentelemetry::Context as OtelContext;
use std::process::exit;
use tracing::{debug, info, instrument, warn};

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

use crate::util::exitcode;

/// Top-level command, with:
///  - Global arguments
///  - A specific subcommand
#[derive(Debug, Parser)]
#[command(
name = BrandingCompileEnvVars::bin_name(),
term_width = 100,
about = docs::about(ABOUT),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
version,
long_version = Version::clappy(),
next_help_heading = "Global Options",
disable_help_flag = true,
)]
pub struct OckamCommand {
    #[command(subcommand)]
    pub(crate) subcommand: OckamSubcommand,

    #[command(flatten)]
    global_args: GlobalArgs,
}

impl OckamCommand {
    async fn init_cli_state(&self, in_memory: bool) -> CliState {
        match CliState::new(in_memory).await {
            Ok(state) => state,
            Err(err) => {
                // If the user is trying to run `ockam reset` and the local state is corrupted,
                // we can try to hard reset the local state.
                if let OckamSubcommand::Reset(c) = &self.subcommand {
                    c.hard_reset();
                    println!(
                        "{}",
                        fmt_ok!(
                            "Local {} configuration deleted",
                            BrandingCompileEnvVars::bin_name()
                        )
                    );
                    exit(exitcode::OK);
                }
                eprintln!("{}", fmt_err!("Failed to initialize local state"));
                eprintln!(
                    "{}",
                    fmt_log!(
                        "Consider upgrading to the latest version of {} Command",
                        BrandingCompileEnvVars::bin_name()
                    )
                );
                let ockam_home = std::env::var("OCKAM_HOME").unwrap_or("~/.ockam".to_string());
                eprintln!(
                    "{}",
                    fmt_log!(
                        "You can also try removing the local state using {} \
                        or deleting the directory at {}",
                        color_primary("ockam reset"),
                        color_primary(ockam_home)
                    )
                );
                eprintln!("\n{:?}", miette!(err.to_string()));
                exit(exitcode::SOFTWARE);
            }
        }
    }

    /// Set up a logger and a tracer for the current node
    /// If the node is a background node we always enable logging, regardless of environment variables
    fn setup_logging_tracing(
        &self,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &ExportingConfiguration,
    ) -> Option<TracingGuard> {
        if !logging_configuration.is_enabled() && !tracing_configuration.is_enabled() {
            return None;
        };

        let app_name = if self.subcommand.is_background_node() {
            "local node"
        } else {
            "cli"
        };
        let tracing_guard = LoggingTracing::setup(
            logging_configuration,
            tracing_configuration,
            app_name,
            self.subcommand.node_name(),
        );

        Some(tracing_guard)
    }

    /// Create the logging configuration, depending on the command to execute
    fn make_logging_configuration(&self, is_tty: bool) -> miette::Result<LoggingConfiguration> {
        let log_path = self.subcommand.log_path();
        if self.subcommand.is_background_node() {
            Ok(LoggingConfiguration::background(log_path).into_diagnostic()?)
        } else {
            let level_and_crates = LogLevelWithCratesFilter::from_verbose(self.global_args.verbose)
                .into_diagnostic()?;
            let log_path =
                if level_and_crates.explicit_verbose_flag || self.subcommand.is_foreground_node() {
                    None
                } else {
                    Some(CliState::command_log_path(self.subcommand.name().as_str())?)
                };
            let colored = if !self.global_args.no_color && is_tty && log_path.is_none() {
                Colored::On
            } else {
                Colored::Off
            };
            Ok(logging_configuration(level_and_crates, log_path, colored).into_diagnostic()?)
        }
    }

    /// Create the tracing configuration, depending on the command to execute
    async fn make_tracing_configuration(
        &self,
        state: &CliState,
    ) -> miette::Result<ExportingConfiguration> {
        if self.subcommand.is_background_node() {
            ExportingConfiguration::background(state)
                .await
                .into_diagnostic()
        } else {
            ExportingConfiguration::foreground(state)
                .await
                .into_diagnostic()
        }
    }

    /// Run the command
    pub async fn run(self, ctx: &Context, arguments: &[String]) -> miette::Result<()> {
        // If test_argument_parser is true, command arguments are checked
        // but the command is not executed. This is useful to test arguments
        // without having to execute their logic.
        if self.global_args.test_argument_parser {
            return Ok(());
        }

        load_compile_time_vars();

        // Sets a hook using our own Error Report Handler.
        // This allows us to customize how we format the error messages and their content.
        let _hook_result = miette::set_hook(Box::new(|_| Box::new(ErrorReportHandler::new())));

        let command_name = self.subcommand.name();

        let mut in_memory = false;

        if let OckamSubcommand::Node(cmd) = &self.subcommand {
            if let crate::node::NodeSubcommand::Create(c) = &cmd.subcommand {
                in_memory = c.in_memory;
            }
        }

        let logging_configuration = self.make_logging_configuration(Term::stdout().is_term())?;

        let (tracing_configuration, tracing_guard, cli_state) = if !is_exporting_set()? {
            // Allows to have logging enabled before initializing CliState
            let tracing_configuration = ExportingConfiguration::off().into_diagnostic()?;
            let tracing_guard =
                self.setup_logging_tracing(&logging_configuration, &tracing_configuration);

            (tracing_configuration, tracing_guard, None)
        } else {
            let cli_state = self.init_cli_state(in_memory).await;
            let tracing_configuration = self.make_tracing_configuration(&cli_state).await?;
            let tracing_guard =
                self.setup_logging_tracing(&logging_configuration, &tracing_configuration);
            let cli_state = cli_state.set_tracing_enabled(tracing_configuration.is_enabled());

            (tracing_configuration, tracing_guard, Some(cli_state))
        };

        info!("Tracing initialized");
        debug!("{:#?}", logging_configuration);
        debug!("{:#?}", tracing_configuration);

        let tracer = global::tracer(OCKAM_TRACER_NAME);

        let span = if let Some(opentelemetry_context) = self.subcommand.get_opentelemetry_context()
        {
            let span_builder =
                SpanBuilder::from_name(command_name.clone()).with_links(vec![Link::new(
                    opentelemetry_context
                        .extract()
                        .span()
                        .span_context()
                        .clone(),
                    vec![],
                    0,
                )]);
            tracer.build(span_builder)
        } else {
            tracer.start(command_name.clone())
        };
        let cx = OtelContext::current_with_span(span);

        // TODO: Add another span here?
        let cli_state = match cli_state {
            Some(cli_state) => cli_state,
            None => self
                .init_cli_state(in_memory)
                .with_context(cx.clone())
                .await
                .set_tracing_enabled(tracing_configuration.is_enabled()),
        };

        let terminal = Terminal::new(
            logging_configuration.is_enabled(),
            logging_configuration.log_dir().is_some(),
            self.global_args.quiet,
            self.global_args.no_color,
            self.global_args.no_input,
            self.global_args.output_format(),
            BrandingCompileEnvVars::bin_name(),
            BrandingCompileEnvVars::brand_name(),
        );

        let options = CommandGlobalOpts::new(self.global_args.clone(), cli_state, terminal);

        options.log_inputs(arguments, &self.subcommand);

        if let Err(err) = check_if_an_upgrade_is_available(&options).await {
            warn!("Failed to check for upgrade, error={err}");
            options
                .terminal
                .write_line(fmt_warn!("Failed to check for upgrade"))?;
        }

        let result = self
            .run_command(ctx, options.clone(), &command_name, arguments)
            .with_context(cx)
            .await;

        if let Err(ref e) = result {
            add_command_error_event(
                options.state.clone(),
                &command_name,
                &format!("{e}"),
                arguments.join(" "),
            )
            .await?;
        };

        if let Some(tracing_guard) = tracing_guard {
            tracing_guard.force_flush().await;
            tracing_guard.shutdown().await;
        };

        result
    }

    #[instrument(skip_all, fields(command = self.subcommand.name()))]
    async fn run_command(
        self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        command_name: &str,
        arguments: &[String],
    ) -> miette::Result<()> {
        add_command_event(opts.state.clone(), command_name, arguments.join(" ")).await?;
        self.subcommand.run(ctx, opts).await
    }
}
