use crate::base_command::BaseCommand;
use crate::branding::{load_compile_time_vars, BrandingCompileEnvVars, OUTPUT_BRANDING};
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
    is_exporting_set, logging_configuration, logging_enabled, Colored, CratesFilter,
    ExportingConfiguration, LogFormat, LogLevelWithCratesFilter, LoggingConfiguration,
    LoggingEnabled, LoggingTracing, OckamUserLogFormat, TracingGuard,
};
use ockam_api::terminal::{LoggingOptions, Terminal};
use ockam_api::{fmt_err, fmt_log, fmt_ok, fmt_warn, CliState};
use ockam_core::OCKAM_TRACER_NAME;
use ockam_node::Context;
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Link, SpanBuilder, TraceContextExt, Tracer};
use opentelemetry::Context as OtelContext;
use std::process::exit;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn, Level};

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

use crate::util::exitcode;

/// Top-level command, with:
///  - Global arguments
///  - A specific subcommand
#[derive(Debug, Parser, Default)]
#[command(
name = BrandingCompileEnvVars::bin_name(),
term_width = 80,
about = docs::about(ABOUT),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
version,
long_version = Version::clappy(),
next_help_heading = "Global Options",
disable_help_flag = true,
arg_required_else_help = false,
subcommand_required = false,
)]
pub struct OckamCommand {
    #[command(flatten)]
    pub global_args: GlobalArgs,

    #[command(flatten)]
    pub base_command_args: BaseCommand,

    #[command(subcommand)]
    pub(crate) subcommand: Option<OckamSubcommand>,
}

impl OckamCommand {
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

        let mut in_memory = false;
        if let Some(OckamSubcommand::Node(cmd)) = &self.subcommand {
            if let crate::node::NodeSubcommand::Create(c) = &cmd.subcommand {
                in_memory = c.in_memory;
            }
        }

        let logging_configuration = self.make_logging_configuration(Term::stdout().is_term())?;

        let (exporting_configuration, tracing_guard, cli_state) = if is_exporting_set()? {
            let cli_state = self.init_cli_state(in_memory).await;
            let exporting_configuration =
                self.make_exporting_configuration(&cli_state, ctx).await?;
            let cli_state = cli_state.set_tracing_enabled(exporting_configuration.is_enabled());
            let cli_state = Arc::new(cli_state);
            let tracing_guard = self.setup_logging_tracing(
                cli_state.clone(),
                &logging_configuration,
                &exporting_configuration,
                ctx,
            );

            (exporting_configuration, tracing_guard, Some(cli_state))
        } else {
            // Allows having logging enabled before initializing CliState
            let exporting_configuration = ExportingConfiguration::off().into_diagnostic()?;
            let cli_state = self.init_cli_state(true).await;
            let tracing_guard = self.setup_logging_tracing(
                Arc::new(cli_state),
                &logging_configuration,
                &exporting_configuration,
                ctx,
            );

            (exporting_configuration, tracing_guard, None)
        };

        info!("Tracing initialized");
        debug!("{:#?}", logging_configuration);
        debug!("{:#?}", exporting_configuration);

        let command_name = self.name();
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let span = if let Some(opentelemetry_context) = self
            .subcommand
            .as_ref()
            .and_then(|c| c.get_opentelemetry_context())
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

        let cli_state = match cli_state {
            Some(cli_state) => cli_state,
            None => Arc::new(
                self.init_cli_state(in_memory)
                    .with_context(cx.clone())
                    .await
                    .set_tracing_enabled(exporting_configuration.is_enabled()),
            ),
        };

        let terminal = Terminal::new(
            LoggingOptions {
                enabled: logging_configuration.is_enabled(),
                logging_to_file: logging_configuration.log_dir().is_some(),
                with_user_format: logging_configuration.format() == LogFormat::User,
            },
            self.global_args.quiet,
            self.global_args.no_color,
            self.global_args.no_input,
            self.global_args.output_format(),
            OUTPUT_BRANDING.clone(),
        );

        let options = CommandGlobalOpts::new(self.global_args.clone(), cli_state, terminal);
        options.log_inputs(arguments, &self.name());

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

    async fn init_cli_state(&self, in_memory: bool) -> CliState {
        match CliState::new(in_memory).await {
            Ok(state) => state,
            Err(err) => {
                // If the user is trying to run `ockam reset` and the local state is corrupted,
                // we can try to hard reset the local state.
                if let Some(OckamSubcommand::Reset(c)) = &self.subcommand {
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
                let ockam_home = std::env::var("OCKAM_HOME")
                    .unwrap_or(BrandingCompileEnvVars::home_dir().to_string());
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
        cli_state: Arc<CliState>,
        logging_configuration: &LoggingConfiguration,
        exporting_configuration: &ExportingConfiguration,
        ctx: &Context,
    ) -> Option<TracingGuard> {
        if !logging_configuration.is_enabled() && !exporting_configuration.is_enabled() {
            return None;
        };

        let app_name = if self
            .subcommand
            .as_ref()
            .map(|c| c.is_local_node())
            .unwrap_or_default()
        {
            "local node"
        } else {
            "cli"
        };
        let tracing_guard = LoggingTracing::setup(
            cli_state,
            logging_configuration,
            exporting_configuration,
            app_name,
            self.subcommand.as_ref().and_then(|c| c.node_name()),
            ctx,
        );

        Some(tracing_guard)
    }

    /// Create the logging configuration, depending on the command to execute
    fn make_logging_configuration(&self, is_tty: bool) -> miette::Result<LoggingConfiguration> {
        let subcommand = self.subcommand.as_ref();

        if subcommand
            .map(|c| c.is_background_node())
            .unwrap_or_default()
        {
            let log_path = subcommand.map(|c| c.log_path()).unwrap_or_default();
            Ok(LoggingConfiguration::background(log_path).into_diagnostic()?)
        } else {
            let verbose = self.global_args.verbose;
            let mut level_and_crates =
                LogLevelWithCratesFilter::from_verbose(verbose).into_diagnostic()?;
            let mut log_path = if level_and_crates.explicit_verbose_flag {
                None
            } else {
                Some(CliState::command_log_path(self.name().as_str())?)
            };
            let mut logging_enabled = logging_enabled()?;
            let mut default_log_format = LogFormat::Default;
            let is_foreground_node = subcommand
                .map(|c| c.is_foreground_node())
                .unwrap_or_default();
            if is_foreground_node && verbose == 0 {
                log_path = None;
                logging_enabled = LoggingEnabled::On;
                level_and_crates.crates_filter =
                    CratesFilter::Selected(vec![OckamUserLogFormat::TARGET.to_string()]);
                default_log_format = LogFormat::User;
            }
            let colored = if !self.global_args.no_color && is_tty && log_path.is_none() {
                Colored::On
            } else {
                Colored::Off
            };
            Ok(logging_configuration(
                level_and_crates,
                log_path,
                colored,
                default_log_format,
                logging_enabled,
            )
            .into_diagnostic()?)
        }
    }

    /// Create the exporting configuration, depending on the command to execute
    async fn make_exporting_configuration(
        &self,
        state: &CliState,
        ctx: &Context,
    ) -> miette::Result<ExportingConfiguration> {
        if self
            .subcommand
            .as_ref()
            .map(|c| c.is_background_node())
            .unwrap_or_default()
        {
            ExportingConfiguration::background(state, ctx)
                .await
                .into_diagnostic()
        } else {
            ExportingConfiguration::foreground(state, ctx)
                .await
                .into_diagnostic()
        }
    }

    #[instrument(skip_all, fields(command = self.name()), level = Level::TRACE)]
    async fn run_command(
        self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        command_name: &str,
        arguments: &[String],
    ) -> miette::Result<()> {
        add_command_event(opts.state.clone(), command_name, arguments.join(" ")).await?;
        if let Some(subcommand) = self.subcommand {
            subcommand.run(ctx, opts).await
        } else {
            self.base_command_args.run(ctx, opts).await
        }
    }

    fn name(&self) -> String {
        if let Some(subcommand) = &self.subcommand {
            subcommand.name()
        } else {
            self.base_command_args.name()
        }
    }
}
