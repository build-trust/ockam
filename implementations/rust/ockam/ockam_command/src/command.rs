use crate::command_events::{add_command_error_event, add_command_event};
use crate::command_global_opts::CommandGlobalOpts;
use crate::docs;
use crate::global_args::GlobalArgs;
use crate::subcommand::OckamSubcommand;
use crate::upgrade::check_if_an_upgrade_is_available;
use crate::version::Version;
use crate::{fmt_warn, OckamColor};

use clap::Parser;
use colorful::Colorful;
use miette::GraphicalReportHandler;
use ockam_core::OCKAM_TRACER_NAME;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use r3bl_rs_utils_core::UnicodeString;
use r3bl_tui::{
    ColorWheel, ColorWheelConfig, ColorWheelSpeed, GradientGenerationPolicy, TextColorizationPolicy,
};
use tracing::{instrument, warn};

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Top-level command, with:
///
///  - Global arguments (which apply to any OckamSubcommand)
///  - A specific subcommand
///
#[derive(Debug, Parser)]
#[command(
name = "ockam",
term_width = 100,
about = docs::about(ABOUT),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
version,
long_version = Version::long(),
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
    /// Run the command
    pub fn run(self, arguments: Vec<String>) -> miette::Result<()> {
        // If test_argument_parser is true, command arguments are checked
        // but the command is not executed. This is useful to test arguments
        // without having to execute their logic.
        if self.global_args.test_argument_parser {
            return Ok(());
        }

        // Sets a hook using our own Error Report Handler
        // This allows us to customize how we
        // format the error messages and their content.
        let _hook_result = miette::set_hook(Box::new(|_| {
            Box::new(
                GraphicalReportHandler::new()
                    .with_cause_chain()
                    .with_footer(Version::short().light_gray().to_string())
                    .with_urls(false),
            )
        }));
        let options = CommandGlobalOpts::new(&arguments, &self.global_args, &self.subcommand)?;

        if let Err(err) = check_if_an_upgrade_is_available(&options) {
            warn!("Failed to check for upgrade, error={err}");
            options
                .terminal
                .write_line(&fmt_warn!("Failed to check for upgrade"))
                .unwrap();
        }

        // Display Header if needed
        if self.subcommand.should_display_header() {
            let ockam_header = include_str!("../static/ockam_ascii.txt").trim();
            let gradient_steps = Vec::from(
                [
                    OckamColor::OckamBlue.value(),
                    OckamColor::HeaderGradient.value(),
                ]
                .map(String::from),
            );
            let colored_header = ColorWheel::new(vec![ColorWheelConfig::Rgb(
                gradient_steps,
                ColorWheelSpeed::Medium,
                50,
            )])
            .colorize_into_string(
                &UnicodeString::from(ockam_header),
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );

            let _ = options
                .terminal
                .write_line(&format!("{}\n", colored_header));
        }

        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let command_name = self.subcommand.name();
        let result =
            if let Some(opentelemetry_context) = self.subcommand.get_opentelemetry_context() {
                let span = tracer
                    .start_with_context(command_name.clone(), &opentelemetry_context.extract());
                let cx = Context::current_with_span(span);
                let _guard = cx.clone().attach();
                self.run_command(options.clone(), &command_name, &arguments)
            } else {
                tracer.in_span(self.subcommand.name(), |_| {
                    self.run_command(options.clone(), &command_name, &arguments)
                })
            };
        if let Err(ref e) = result {
            add_command_error_event(
                options.state.clone(),
                &command_name,
                &format!("{e}"),
                arguments.join(" "),
            )?
        };
        options.shutdown();
        result
    }

    #[instrument(skip_all, fields(command = self.subcommand.name()))]
    fn run_command(
        self,
        opts: CommandGlobalOpts,
        command_name: &str,
        arguments: &[String],
    ) -> miette::Result<()> {
        add_command_event(opts.state.clone(), command_name, arguments.join(" "))?;
        self.subcommand.run(opts)
    }
}
