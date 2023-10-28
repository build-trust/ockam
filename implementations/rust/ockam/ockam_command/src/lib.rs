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

mod admin;
mod authenticated;
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
mod trust_context;
mod upgrade;
pub mod util;
mod vault;
mod version;
mod worker;

use crate::admin::AdminCommand;
use crate::authority::AuthorityCommand;
use crate::flow_control::FlowControlCommand;
use crate::logs::setup_logging;
use crate::node::NodeSubcommand;
use crate::run::RunCommand;
use crate::subscription::SubscriptionCommand;
pub use crate::terminal::{OckamColor, Terminal, TerminalStream};
use authenticated::AuthenticatedCommand;
use clap::{ArgAction, Args, Parser, Subcommand};

use crate::kafka::direct::KafkaDirectCommand;
use crate::kafka::outlet::KafkaOutletCommand;
use crate::output::{Output, OutputFormat};
use crate::sidecar::SidecarCommand;
use colorful::Colorful;
use completion::CompletionCommand;
use configuration::ConfigurationCommand;
use console::Term;
use credential::CredentialCommand;
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
use miette::GraphicalReportHandler;
use node::NodeCommand;
use ockam_api::cli_state::CliState;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use policy::PolicyCommand;
use project::ProjectCommand;
use relay::RelayCommand;
use reset::ResetCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
#[cfg(feature = "orchestrator")]
use share::ShareCommand;
use space::SpaceCommand;
use status::StatusCommand;
use std::{path::PathBuf, sync::Mutex};
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use trust_context::TrustContextCommand;
use upgrade::check_if_an_upgrade_is_available;
use util::{exitcode, exitcode::ExitCode};
use vault::VaultCommand;
use version::Version;
use worker::WorkerCommand;

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

static PARSER_LOGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

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

#[derive(Clone)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub state: CliState,
    pub terminal: Terminal<TerminalStream<Term>>,
}

impl CommandGlobalOpts {
    pub fn new(global_args: GlobalArgs) -> Self {
        let state = CliState::initialize().unwrap_or_else(|_| {
            let state = CliState::backup_and_reset().expect(
                "Failed to initialize CliState. Try to manually remove the '~/.ockam' directory",
            );
            let dir = &state.dir;
            let backup_dir = CliState::backup_default_dir().unwrap();
            eprintln!(
                "The {dir:?} directory has been reset and has been backed up to {backup_dir:?}"
            );
            state
        });
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
        }
    }

    pub fn set_quiet(&self) -> Self {
        let mut clone = self.clone();
        clone.global_args = clone.global_args.set_quiet();
        clone.terminal = clone.terminal.set_quiet();
        clone
    }

    /// Print a value on the console.
    /// TODO: replace this implementation with a call to the terminal instead
    pub fn println<T>(&self, t: &T) -> Result<()>
    where
        T: Output + serde::Serialize,
    {
        self.global_args.output_format.println_value(t)
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
    Authenticated(AuthenticatedCommand),
    Configuration(ConfigurationCommand),

    Completion(CompletionCommand),
    Markdown(MarkdownCommand),
    Manpages(ManpagesCommand),
    TrustContext(TrustContextCommand),
    Environment(EnvironmentCommand),

    FlowControl(FlowControlCommand),
}

impl OckamSubcommand {
    pub fn should_display_header(&self) -> bool {
        // Currently only enroll command displays the header
        matches!(self, OckamSubcommand::Enroll(_))
    }
}

pub fn run() {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();

    match OckamCommand::try_parse_from(input) {
        Ok(command) => {
            if !command.global_args.test_argument_parser {
                check_if_an_upgrade_is_available();
            }

            command.run();
        }
        Err(help) => pager::render_help(help),
    };
}

impl OckamCommand {
    pub fn run(self) {
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
        let options = CommandGlobalOpts::new(self.global_args.clone());

        let _tracing_guard = if !options.global_args.quiet {
            let log_path = self.log_path(&options);
            let guard = setup_logging(
                options.global_args.verbose,
                options.global_args.no_color,
                options.terminal.is_tty(),
                log_path,
            );
            tracing::debug!("{}", Version::short());
            tracing::debug!("Parsed {:?}", &self);
            guard
        } else {
            None
        };

        // If test_argument_parser is true, command arguments are checked
        // but the command is not executed. This is useful to test arguments
        // without having to execute their logic.
        if options.global_args.test_argument_parser {
            return;
        }

        // Display Header if needed
        if self.subcommand.should_display_header() {
            let ockam_header = include_str!("../static/ockam_ascii.txt").trim();
            let colored_header = ockam_header.gradient_with_color(
                OckamColor::OckamBlue.color(),
                OckamColor::HeaderGradient.color(),
            );

            let _ = options
                .terminal
                .write_line(&format!("{}\n", colored_header));
        }

        match self.subcommand {
            OckamSubcommand::Enroll(c) => c.run(options),
            OckamSubcommand::Space(c) => c.run(options),
            OckamSubcommand::Project(c) => c.run(options),
            OckamSubcommand::Admin(c) => c.run(options),
            #[cfg(feature = "orchestrator")]
            OckamSubcommand::Share(c) => c.run(options),
            OckamSubcommand::Subscription(c) => c.run(options),

            OckamSubcommand::Node(c) => c.run(options),
            OckamSubcommand::Worker(c) => c.run(options),
            OckamSubcommand::Service(c) => c.run(options),
            OckamSubcommand::Message(c) => c.run(options),
            OckamSubcommand::Relay(c) => c.run(options),

            OckamSubcommand::KafkaOutlet(c) => c.run(options),
            OckamSubcommand::TcpListener(c) => c.run(options),
            OckamSubcommand::TcpConnection(c) => c.run(options),
            OckamSubcommand::TcpOutlet(c) => c.run(options),
            OckamSubcommand::TcpInlet(c) => c.run(options),

            OckamSubcommand::KafkaConsumer(c) => c.run(options),
            OckamSubcommand::KafkaProducer(c) => c.run(options),
            OckamSubcommand::KafkaDirect(c) => c.run(options),

            OckamSubcommand::SecureChannelListener(c) => c.run(options),
            OckamSubcommand::SecureChannel(c) => c.run(options),

            OckamSubcommand::Vault(c) => c.run(options),
            OckamSubcommand::Identity(c) => c.run(options),
            OckamSubcommand::Credential(c) => c.run(options),
            OckamSubcommand::Authority(c) => c.run(options),
            OckamSubcommand::Policy(c) => c.run(options),
            OckamSubcommand::Lease(c) => c.run(options),

            OckamSubcommand::Run(c) => c.run(options),
            OckamSubcommand::Status(c) => c.run(options),
            OckamSubcommand::Reset(c) => c.run(options),
            OckamSubcommand::Authenticated(c) => c.run(options),
            OckamSubcommand::Configuration(c) => c.run(options),

            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Markdown(c) => c.run(),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::TrustContext(c) => c.run(options),
            OckamSubcommand::Environment(c) => c.run(),

            OckamSubcommand::FlowControl(c) => c.run(options),
            OckamSubcommand::Sidecar(c) => c.run(options),
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
                let path = opts
                    .state
                    .nodes
                    .stdout_logs(&c.node_name)
                    .unwrap_or_else(|_| {
                        panic!("Failed to initialize logs file for node {}", c.node_name)
                    });
                return Some(path);
            }
        }
        None
    }
}

/// Display and clear any known messages from parsing.
pub(crate) fn display_parse_logs(opts: &CommandGlobalOpts) {
    if let Ok(mut logs) = PARSER_LOGS.lock() {
        logs.iter().for_each(|msg| {
            let _ = opts.terminal.write_line(msg);
        });

        logs.clear();
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
