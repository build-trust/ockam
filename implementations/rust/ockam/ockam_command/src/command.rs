use crate::command_events::{add_command_error_event, add_command_event};
use crate::command_global_opts::CommandGlobalOpts;
use crate::global_args::GlobalArgs;
use crate::subcommand::OckamSubcommand;
use crate::upgrade::check_if_an_upgrade_is_available;
use crate::version::Version;
use crate::{docs, ErrorReportHandler};
use clap::Parser;
use colorful::Colorful;
use ockam_api::fmt_warn;
use ockam_core::OCKAM_TRACER_NAME;
use ockam_node::database::OCKAM_SQLITE_IN_MEMORY;
use opentelemetry::trace::{Link, SpanBuilder, TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use tracing::{instrument, warn};

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

pub use crate::environment::compile_time_vars::{
    BIN_NAME, BRAND_NAME, OCKAM_COMMANDS, OCKAM_COMMAND_BIN_NAME, OCKAM_COMMAND_BRAND_NAME,
    OCKAM_COMMAND_SUPPORT_EMAIL,
};

/// Top-level command, with:
///  - Global arguments
///  - A specific subcommand
#[derive(Debug, Parser)]
#[command(
name = BIN_NAME,
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
    /// Run the command
    pub fn run(self, arguments: Vec<String>) -> miette::Result<()> {
        // If test_argument_parser is true, command arguments are checked
        // but the command is not executed. This is useful to test arguments
        // without having to execute their logic.
        if self.global_args.test_argument_parser {
            return Ok(());
        }

        // Sets a hook using our own Error Report Handler.
        // This allows us to customize how we format the error messages and their content.
        let _hook_result = miette::set_hook(Box::new(|_| Box::new(ErrorReportHandler::new())));

        let command_name = self.subcommand.name();

        // Set the in-memory env var if needed
        if let OckamSubcommand::Node(cmd) = &self.subcommand {
            if let crate::node::NodeSubcommand::Create(c) = &cmd.subcommand {
                if c.in_memory {
                    std::env::set_var(OCKAM_SQLITE_IN_MEMORY, "true");
                }
            }
        }

        let options = CommandGlobalOpts::new(&arguments, &self.global_args, &self.subcommand)?;

        if let Err(err) = check_if_an_upgrade_is_available(&options) {
            warn!("Failed to check for upgrade, error={err}");
            options
                .terminal
                .write_line(fmt_warn!("Failed to check for upgrade"))?;
        }

        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let result =
            if let Some(opentelemetry_context) = self.subcommand.get_opentelemetry_context() {
                let context = Context::current();
                let span_builder = SpanBuilder::from_name(command_name.clone().to_string())
                    .with_links(vec![Link::new(
                        opentelemetry_context
                            .extract()
                            .span()
                            .span_context()
                            .clone(),
                        vec![],
                        0,
                    )]);
                let span = tracer.build_with_context(span_builder, &context);
                let cx = Context::current_with_span(span);
                let _guard = cx.clone().attach();
                self.run_command(options.clone(), &command_name, &arguments)
            } else {
                tracer.in_span(command_name.clone(), |_| {
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
