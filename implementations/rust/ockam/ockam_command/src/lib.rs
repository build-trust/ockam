//! This crate provides the ockam command line application to:
//!  - start Ockam nodes and interact with them
//!  - manage projects and spaces hosted within the Ockam Orchestrator
//!
//! For more information please visit the [command guide](https://docs.ockam.io/reference/command)
//!
//! ## Instructions on how to install Ockam Command
//! 1. You can install Ockam Command pre-built binary using these [steps](https://docs.ockam.io/#quick-start). You can run the following command in your terminal to install the pre-built binary:
//!
//!     ```bash
//!     curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
//!     ```
//!
//! 1. To build Ockam Command from source, fork the [repo](https://github.com/build-trust/ockam), and then clone it to your machine. Open a terminal and go to the folder that you just cloned the repo into. Then run the following to install `ockam` so that you can run it from the command line.
//!
//!     ```bash
//!     cd implementations/rust/ockam/ockam_command && cargo install --path .
//!     ```

use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;

use clap::{ArgAction, Args, Parser, Subcommand};
use colorful::Colorful;
use console::Term;
use miette::{miette, GraphicalReportHandler};
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{global, Context};
use tokio::runtime::Runtime;
use tracing::{error, instrument};

use completion::CompletionCommand;
use configuration::ConfigurationCommand;
use enroll::EnrollCommand;
use environment::EnvironmentCommand;
use error::{Error, Result};
use identity::IdentityCommand;
use kafka::consumer::KafkaConsumerCommand;
use kafka::producer::KafkaProducerCommand;
use lease::LeaseCommand;
use manpages::ManpagesCommand;
use markdown::MarkdownCommand;
use message::MessageCommand;
use node::NodeCommand;
use ockam_api::cli_state::CliState;
use ockam_api::logs::{TracingGuard, OCKAM_TRACER_NAME};
use ockam_core::env::get_env_with_default;
use ockam_node::{Executor, OpenTelemetryContext};
use policy::PolicyCommand;
use project::ProjectCommand;
use r3bl_rs_utils_core::UnicodeString;
use r3bl_tui::{
    ColorWheel, ColorWheelConfig, ColorWheelSpeed, GradientGenerationPolicy, TextColorizationPolicy,
};
use relay::RelayCommand;
use reset::ResetCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
#[cfg(feature = "orchestrator")]
use share::ShareCommand;
use space::SpaceCommand;
use status::StatusCommand;
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use tracing::warn;
use upgrade::check_if_an_upgrade_is_available;
use util::{exitcode, exitcode::ExitCode};
use vault::VaultCommand;
use version::Version;
use worker::WorkerCommand;

use crate::admin::AdminCommand;
use crate::authority::{AuthorityCommand, AuthoritySubcommand};
use crate::credential::CredentialCommand;
use crate::flow_control::FlowControlCommand;
use crate::kafka::direct::KafkaDirectCommand;
use crate::kafka::outlet::KafkaOutletCommand;
use crate::logs::setup_logging_tracing;
use crate::node::NodeSubcommand;
use crate::output::OutputFormat;
use crate::run::RunCommand;
use crate::sidecar::SidecarCommand;
use crate::subscription::SubscriptionCommand;
use crate::terminal::color_primary;
pub use crate::terminal::{OckamColor, Terminal, TerminalStream};

mod admin;
mod authority;
mod completion;
mod configuration;
mod credential;
mod docs;
pub mod enroll;
mod environment;
pub mod error;
mod flow_control;
pub mod identity;
mod kafka;
mod lease;
mod logs;
mod manpages;
mod markdown;
mod message;
pub mod node;
mod operation;
mod output;
mod pager;
mod policy;
mod project;
mod relay;
mod reset;
mod run;
mod secure_channel;
mod service;
#[cfg(feature = "orchestrator")]
mod share;
pub mod shutdown;
mod sidecar;
mod space;
mod status;
mod subscription;
pub mod tcp;
mod terminal;
mod upgrade;
pub mod util;
mod vault;
mod version;
mod worker;

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

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
    subcommand: OckamSubcommand,

    #[command(flatten)]
    global_args: GlobalArgs,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    #[arg(
    global = true,
    long,
    short,
    help("Print help information (-h compact, --help extensive)"),
    long_help("Print help information (-h displays compact help summary, --help displays extensive help summary"),
    help_heading("Global Options"),
    action = ArgAction::Help
    )]
    help: Option<bool>,

    /// Do not print any log messages
    #[arg(global = true, long, short, default_value_t = quiet_default_value())]
    quiet: bool,

    /// Increase verbosity of trace messages
    #[arg(
    global = true,
    long,
    short,
    long_help("Increase verbosity of trace messages by repeating the flag. Use `-v` to show \
    info messages, `-vv` to show debug messages, and `-vvv` to show trace messages"),
    action = ArgAction::Count
    )]
    verbose: u8,

    /// Output without any colors
    #[arg(hide = docs::hide(), global = true, long, default_value_t = no_color_default_value())]
    no_color: bool,

    /// Disable tty functionality
    #[arg(hide = docs::hide(), global = true, long, default_value_t = no_input_default_value())]
    no_input: bool,

    /// Output format
    #[arg(
    hide = docs::hide(),
    global = true,
    long = "output",
    value_enum,
    default_value = "plain"
    )]
    output_format: OutputFormat,

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[arg(global = true, long, hide = true)]
    test_argument_parser: bool,
}

fn quiet_default_value() -> bool {
    get_env_with_default("QUIET", false).unwrap_or(false)
}

fn no_color_default_value() -> bool {
    get_env_with_default("NO_COLOR", false).unwrap_or(false)
}

fn no_input_default_value() -> bool {
    get_env_with_default("NO_INPUT", false).unwrap_or(false)
}

impl Default for GlobalArgs {
    fn default() -> Self {
        Self {
            help: None,
            quiet: quiet_default_value(),
            verbose: 0,
            no_color: no_color_default_value(),
            no_input: no_input_default_value(),
            output_format: OutputFormat::Plain,
            test_argument_parser: false,
        }
    }
}

impl GlobalArgs {
    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.quiet = true;
        clone
    }
}

#[derive(Clone, Debug)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub state: CliState,
    pub terminal: Terminal<TerminalStream<Term>>,
    pub rt: Arc<Runtime>,
}

impl CommandGlobalOpts {
    pub fn new(global_args: GlobalArgs, cmd: &OckamSubcommand) -> Self {
        let terminal = Terminal::from(&global_args);
        let state = match CliState::with_default_dir() {
            Ok(state) => state,
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
                terminal
                    .write_line(fmt_log!(
                        "You can also try removing the local state using {} \
                        or deleting the directory at {}",
                        color_primary("ockam reset"),
                        color_primary("~/.ockam")
                    ))
                    .unwrap();
                terminal
                    .write_line(format!("\n{:?}", miette!(err.to_string())))
                    .unwrap();
                exit(exitcode::SOFTWARE);
            }
        };
        Self {
            global_args,
            state,
            terminal,
            rt: Arc::new(Runtime::new().expect("cannot initialize the tokio runtime")),
        }
    }

    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.global_args = clone.global_args.set_quiet();
        clone.terminal = clone.terminal.set_quiet();
        clone
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
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum OckamSubcommand {
    #[command(display_order = 800)]
    Enroll(EnrollCommand),
    Space(SpaceCommand),
    Project(ProjectCommand),
    Sidecar(SidecarCommand),
    Admin(AdminCommand),
    #[cfg(feature = "orchestrator")]
    Share(ShareCommand),
    Subscription(SubscriptionCommand),

    Node(Box<NodeCommand>),
    Worker(WorkerCommand),
    Service(ServiceCommand),
    Message(MessageCommand),
    Relay(RelayCommand),

    TcpListener(TcpListenerCommand),
    TcpConnection(TcpConnectionCommand),
    TcpOutlet(TcpOutletCommand),
    TcpInlet(TcpInletCommand),

    KafkaOutlet(KafkaOutletCommand),
    KafkaConsumer(KafkaConsumerCommand),
    KafkaDirect(KafkaDirectCommand),
    KafkaProducer(KafkaProducerCommand),

    SecureChannelListener(SecureChannelListenerCommand),
    SecureChannel(SecureChannelCommand),

    Vault(VaultCommand),
    Identity(IdentityCommand),
    Credential(CredentialCommand),
    Authority(AuthorityCommand),
    Policy(PolicyCommand),
    Lease(LeaseCommand),

    Run(RunCommand),
    Status(StatusCommand),
    Reset(ResetCommand),
    Configuration(ConfigurationCommand),

    Completion(CompletionCommand),
    Markdown(MarkdownCommand),
    Manpages(ManpagesCommand),
    Environment(EnvironmentCommand),

    FlowControl(FlowControlCommand),
}

impl OckamSubcommand {
    pub fn should_display_header(&self) -> bool {
        // Currently only enroll command displays the header
        matches!(self, OckamSubcommand::Enroll(_))
    }

    /// Return the opentelementry context if the command can be executed as the continuation
    /// of an existing trace
    pub fn get_opentelemetry_context(&self) -> Option<OpenTelemetryContext> {
        match self {
            OckamSubcommand::Node(cmd) => match cmd.as_ref() {
                NodeCommand {
                    subcommand: NodeSubcommand::Create(cmd),
                } => cmd.opentelemetry_context.clone(),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        match self {
            OckamSubcommand::Node(c) => c.name(),
            OckamSubcommand::Enroll(c) => c.name(),
            OckamSubcommand::Space(c) => c.name(),
            OckamSubcommand::Project(c) => c.name(),
            OckamSubcommand::Sidecar(c) => c.name(),
            OckamSubcommand::Admin(c) => c.name(),
            OckamSubcommand::Share(c) => c.name(),
            OckamSubcommand::Subscription(c) => c.name(),
            OckamSubcommand::Worker(c) => c.name(),
            OckamSubcommand::Service(c) => c.name(),
            OckamSubcommand::Message(c) => c.name(),
            OckamSubcommand::Relay(c) => c.name(),
            OckamSubcommand::TcpListener(c) => c.name(),
            OckamSubcommand::TcpConnection(c) => c.name(),
            OckamSubcommand::TcpOutlet(c) => c.name(),
            OckamSubcommand::TcpInlet(c) => c.name(),
            OckamSubcommand::KafkaOutlet(c) => c.name(),
            OckamSubcommand::KafkaConsumer(c) => c.name(),
            OckamSubcommand::KafkaDirect(c) => c.name(),
            OckamSubcommand::KafkaProducer(c) => c.name(),
            OckamSubcommand::SecureChannelListener(c) => c.name(),
            OckamSubcommand::SecureChannel(c) => c.name(),
            OckamSubcommand::Vault(c) => c.name(),
            OckamSubcommand::Identity(c) => c.name(),
            OckamSubcommand::Credential(c) => c.name(),
            OckamSubcommand::Authority(c) => c.name(),
            OckamSubcommand::Policy(c) => c.name(),
            OckamSubcommand::Lease(c) => c.name(),
            OckamSubcommand::Run(c) => c.name(),
            OckamSubcommand::Status(c) => c.name(),
            OckamSubcommand::Reset(c) => c.name(),
            OckamSubcommand::Configuration(c) => c.name(),
            OckamSubcommand::Completion(c) => c.name(),
            OckamSubcommand::Markdown(c) => c.name(),
            OckamSubcommand::Manpages(c) => c.name(),
            OckamSubcommand::Environment(c) => c.name(),
            OckamSubcommand::FlowControl(c) => c.name(),
        }
    }
}

pub fn run() -> miette::Result<()> {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();

    match OckamCommand::try_parse_from(input.clone()) {
        Err(help) => {
            // the -h or --help flag must not be interpreted as an error
            if !input.contains(&"-h".to_string()) && !input.contains(&"--help".to_string()) {
                let command = input
                    .iter()
                    .take_while(|a| !a.starts_with('-'))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                let message = format!(
                    "could not parse the command: {}\n{}",
                    command,
                    input.join(" ")
                );
                send_error_message(&command, &message);
            };
            pager::render_help(help);
        }
        Ok(command) => command.run()?,
    };
    Ok(())
}

fn send_error_message(command: &str, message: &str) {
    let message = message.to_string();
    let command = command.to_string();

    let guard = setup_logging_tracing(false, 0, true, false, None);
    let tracer = global::tracer(OCKAM_TRACER_NAME);
    tracer.in_span(format!("'{}' error", command), |_| {
        let state = CliState::with_default_dir();
        Context::current()
            .span()
            .set_status(opentelemetry::trace::Status::error(message.clone()));
        error!("{}", &message);
        let _ = Executor::execute_future(async move {
            state.unwrap().add_journey_error(&command, message).await
        });
    });
    guard.shutdown()
}

impl OckamCommand {
    pub fn run(self) -> miette::Result<()> {
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
        let options = CommandGlobalOpts::new(self.global_args.clone(), &self.subcommand);

        let tracing_guard = if !options.global_args.quiet {
            let log_path = self.log_path(&options);
            let guard = setup_logging_tracing(
                self.subcommand.get_opentelemetry_context().is_some(),
                options.global_args.verbose,
                options.global_args.no_color,
                options.terminal.is_tty(),
                log_path,
            );
            if let Some(opentelemetry_context) = self.subcommand.get_opentelemetry_context() {
                tracing::debug!("{opentelemetry_context}");
            };
            tracing::debug!("{}", Version::short());
            tracing::debug!("Parsed {:#?}", &self);
            Some(Arc::new(guard))
        } else {
            None
        };

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
        let tracing_guard_clone = tracing_guard.clone();
        let result =
            if let Some(opentelemetry_context) = self.subcommand.get_opentelemetry_context() {
                let span = tracer
                    .start_with_context(self.subcommand.name(), &opentelemetry_context.extract());
                let cx = Context::current_with_span(span);
                let _guard = cx.clone().attach();
                self.run_command(options, tracing_guard_clone)
            } else {
                tracer.in_span(self.subcommand.name(), |_| {
                    self.run_command(options, tracing_guard_clone)
                })
            };

        if let Some(tracing_guard) = tracing_guard {
            tracing_guard.shutdown()
        };
        result
    }

    #[instrument(skip_all, fields(command = self.subcommand.name()))]
    fn run_command(
        self,
        opts: CommandGlobalOpts,
        tracing_guard: Option<Arc<TracingGuard>>,
    ) -> miette::Result<()> {
        match self.subcommand {
            OckamSubcommand::Enroll(c) => c.run(opts),
            OckamSubcommand::Space(c) => c.run(opts),
            OckamSubcommand::Project(c) => c.run(opts),
            OckamSubcommand::Admin(c) => c.run(opts),
            #[cfg(feature = "orchestrator")]
            OckamSubcommand::Share(c) => c.run(opts),
            OckamSubcommand::Subscription(c) => c.run(opts),

            OckamSubcommand::Node(c) => c.run(opts, tracing_guard.clone()),
            OckamSubcommand::Worker(c) => c.run(opts),
            OckamSubcommand::Service(c) => c.run(opts),
            OckamSubcommand::Message(c) => c.run(opts),
            OckamSubcommand::Relay(c) => c.run(opts),

            OckamSubcommand::KafkaOutlet(c) => c.run(opts),
            OckamSubcommand::TcpListener(c) => c.run(opts),
            OckamSubcommand::TcpConnection(c) => c.run(opts),
            OckamSubcommand::TcpOutlet(c) => c.run(opts),
            OckamSubcommand::TcpInlet(c) => c.run(opts),

            OckamSubcommand::KafkaConsumer(c) => c.run(opts),
            OckamSubcommand::KafkaProducer(c) => c.run(opts),
            OckamSubcommand::KafkaDirect(c) => c.run(opts),

            OckamSubcommand::SecureChannelListener(c) => c.run(opts),
            OckamSubcommand::SecureChannel(c) => c.run(opts),

            OckamSubcommand::Vault(c) => c.run(opts),
            OckamSubcommand::Identity(c) => c.run(opts),
            OckamSubcommand::Credential(c) => c.run(opts),
            OckamSubcommand::Authority(c) => c.run(opts),
            OckamSubcommand::Policy(c) => c.run(opts),
            OckamSubcommand::Lease(c) => c.run(opts),

            OckamSubcommand::Run(c) => c.run(opts),
            OckamSubcommand::Status(c) => c.run(opts),
            OckamSubcommand::Reset(c) => c.run(opts),
            OckamSubcommand::Configuration(c) => c.run(opts),

            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Markdown(c) => c.run(),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Environment(c) => c.run(),

            OckamSubcommand::FlowControl(c) => c.run(opts),
            OckamSubcommand::Sidecar(c) => c.run(opts),
        }
    }

    fn log_path(&self, opts: &CommandGlobalOpts) -> Option<PathBuf> {
        // If the subcommand is `node create` then return the log path
        // for the node that is being created
        if let OckamSubcommand::Node(c) = &self.subcommand {
            if let NodeSubcommand::Create(c) = &c.subcommand {
                if c.logging_to_stdout() {
                    return None;
                }
                // In the case where a node is explicitly created in foreground mode, we need
                // to initialize the node directories before we can get the log path.
                return Some(opts.state.node_dir(&c.node_name));
            }
        }
        // If the subcommand is `authority create` then return the log path
        // for the node that is being created
        if let OckamSubcommand::Authority(c) = &self.subcommand {
            let AuthoritySubcommand::Create(c) = &c.subcommand;
            if c.logging_to_stdout() {
                return None;
            }
            // In the case where a node is explicitly created in foreground mode, we need
            // to initialize the node directories before we can get the log path.
            return Some(opts.state.node_dir(&c.node_name));
        }
        None
    }
}

pub(crate) fn replace_hyphen_with_stdin(s: String) -> String {
    let input_stream = std::io::stdin();
    if s.contains("/-") {
        let mut buffer = String::new();
        input_stream
            .read_line(&mut buffer)
            .expect("could not read from standard input");
        let args_from_stdin = buffer
            .trim()
            .split('/')
            .filter(|&s| !s.is_empty())
            .fold("".to_owned(), |acc, s| format!("{acc}/{s}"));

        s.replace("/-", &args_from_stdin)
    } else if s.contains("-/") {
        let mut buffer = String::new();
        input_stream
            .read_line(&mut buffer)
            .expect("could not read from standard input");

        let args_from_stdin = buffer
            .trim()
            .split('/')
            .filter(|&s| !s.is_empty())
            .fold("/".to_owned(), |acc, s| format!("{acc}{s}/"));

        s.replace("-/", &args_from_stdin)
    } else {
        s
    }
}
