//! This crate provides the ockam command line application to:
//!  - start Ockam nodes and interact with them
//!  - manage projects and spaces hosted within the Ockam Orchestrator
//!
//! For more information please visit the [command guide](https://docs.ockam.io/reference/command)

mod admin;
mod authenticated;
mod authority;
mod completion;
mod configuration;
mod credential;
mod docs;
mod enroll;
mod error;
mod flow_control;
mod identity;
mod kafka;
mod lease;
mod logs;
mod manpages;
mod markdown;
mod message;
mod node;
mod policy;
mod project;
mod relay;
mod reset;
mod run;
mod secure_channel;
mod service;
mod space;
mod status;
mod subscription;
mod tcp;
mod terminal;
mod trust_context;
mod upgrade;
mod util;
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
use crate::terminal::{Terminal, TerminalStream};
use authenticated::AuthenticatedCommand;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

use colorful::Colorful;
use completion::CompletionCommand;
use configuration::ConfigurationCommand;
use console::Term;
use credential::CredentialCommand;
use enroll::EnrollCommand;
use error::{Error, Result};
use identity::IdentityCommand;
use kafka::consumer::KafkaConsumerCommand;
use kafka::producer::KafkaProducerCommand;
use lazy_static::lazy_static;
use lease::LeaseCommand;
use manpages::ManpagesCommand;
use markdown::MarkdownCommand;
use message::MessageCommand;
use node::NodeCommand;
use ockam_api::cli_state::{CliState, StateDirTrait};
use policy::PolicyCommand;
use project::ProjectCommand;
use relay::RelayCommand;
use reset::ResetCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
use space::SpaceCommand;
use status::StatusCommand;
use std::path::PathBuf;
use std::sync::Mutex;
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use terminal::OckamColor;
use trust_context::TrustContextCommand;
use upgrade::check_if_an_upgrade_is_available;
use util::{exitcode, exitcode::ExitCode, OckamConfig};
use vault::VaultCommand;
use version::Version;
use worker::WorkerCommand;

const ABOUT: &str = include_str!("./static/about.txt");
const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

lazy_static! {
    static ref PARSER_LOGS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

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
disable_help_flag = true
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

    /// Do not print any trace messages
    #[arg(global = true, long, short, conflicts_with("verbose"))]
    quiet: bool,

    /// Increase verbosity of trace messages
    #[arg(
    global = true,
    long,
    short,
    conflicts_with("quiet"),
    action = ArgAction::Count
    )]
    verbose: u8,

    /// Output without any colors
    #[arg(hide = docs::hide(), global = true, long)]
    no_color: bool,

    /// Disable tty functionality
    #[arg(hide = docs::hide(), global = true, long)]
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

impl Default for GlobalArgs {
    fn default() -> Self {
        Self {
            help: None,
            quiet: false,
            verbose: 0,
            no_color: false,
            no_input: false,
            output_format: OutputFormat::Plain,
            test_argument_parser: false,
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum EncodeFormat {
    Plain,
    Hex,
}

#[derive(Clone)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub config: OckamConfig,
    pub state: CliState,
    pub terminal: Terminal<TerminalStream<Term>>,
}

impl CommandGlobalOpts {
    fn new(global_args: GlobalArgs, config: OckamConfig) -> Self {
        let state = CliState::try_default().expect("Failed to load the local Ockam configuration");
        let terminal = Terminal::new(
            global_args.quiet,
            global_args.no_color,
            global_args.no_input,
            global_args.output_format.clone(),
        );
        Self {
            global_args,
            config,
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
    Admin(AdminCommand),
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

    KafkaConsumer(KafkaConsumerCommand),
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

    let command = OckamCommand::parse_from(input);

    if !command.global_args.test_argument_parser {
        check_if_an_upgrade_is_available();
    }

    command.run();
}

impl OckamCommand {
    pub fn run(self) {
        let config = OckamConfig::load().expect("Failed to load config");
        let options = CommandGlobalOpts::new(self.global_args.clone(), config);

        let _tracing_guard = if !options.global_args.quiet {
            let log_path = self.log_path(&options);
            let guard = setup_logging(
                options.global_args.verbose,
                options.global_args.no_color,
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
            OckamSubcommand::Subscription(c) => c.run(options),

            OckamSubcommand::Node(c) => c.run(options),
            OckamSubcommand::Worker(c) => c.run(options),
            OckamSubcommand::Service(c) => c.run(options),
            OckamSubcommand::Message(c) => c.run(options),
            OckamSubcommand::Relay(c) => c.run(options),

            OckamSubcommand::TcpListener(c) => c.run(options),
            OckamSubcommand::TcpConnection(c) => c.run(options),
            OckamSubcommand::TcpOutlet(c) => c.run(options),
            OckamSubcommand::TcpInlet(c) => c.run(options),

            OckamSubcommand::KafkaConsumer(c) => c.run(options),
            OckamSubcommand::KafkaProducer(c) => c.run(options),

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
            OckamSubcommand::Authenticated(c) => c.run(),
            OckamSubcommand::Configuration(c) => c.run(options),

            OckamSubcommand::Completion(c) => c.run(),
            OckamSubcommand::Markdown(c) => c.run(),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::TrustContext(c) => c.run(options),

            OckamSubcommand::FlowControl(c) => c.run(options),
        }
    }

    fn log_path(&self, opts: &CommandGlobalOpts) -> Option<PathBuf> {
        // If the subcommand is `node create` then return the log path
        // for the node that is being created
        if let OckamSubcommand::Node(c) = &self.subcommand {
            if let NodeSubcommand::Create(c) = &c.subcommand {
                if let Ok(s) = opts.state.nodes.get(&c.node_name) {
                    return Some(s.stdout_log());
                }
            }
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
