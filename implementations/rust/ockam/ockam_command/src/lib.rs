//! Orchestrate end-to-end encryption, mutual authentication, key management,
//! credential management, and authorization policy enforcement — at scale.

mod admin;
mod authenticated;
mod completion;
mod configuration;
mod credential;
mod enroll;
mod error;
mod forwarder;
mod help;
mod identity;
mod lease;
mod manpages;
mod message;
mod node;
mod policy;
mod project;
mod reset;
mod secure_channel;
mod service;
mod space;
mod status;
mod subscription;
mod tcp;
mod terminal;
mod upgrade;
mod util;
mod vault;
mod version;

use authenticated::AuthenticatedCommand;
use completion::CompletionCommand;
use configuration::ConfigurationCommand;
use credential::CredentialCommand;
use enroll::EnrollCommand;
use error::{Error, Result};
use forwarder::ForwarderCommand;
use identity::IdentityCommand;
use lease::LeaseCommand;
use manpages::ManpagesCommand;
use message::MessageCommand;
use node::NodeCommand;
use policy::PolicyCommand;
use project::ProjectCommand;
use reset::ResetCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
use space::SpaceCommand;
use status::StatusCommand;
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use util::{exitcode, exitcode::ExitCode, setup_logging, OckamConfig};
use vault::VaultCommand;
use version::Version;

use crate::admin::AdminCommand;
use crate::subscription::SubscriptionCommand;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use ockam_api::cli_state::CliState;
use upgrade::check_if_an_upgrade_is_available;

const ABOUT: &str = include_str!("constants/lib/about.txt");
const HELP_DETAIL: &str = include_str!("constants/lib/help_detail.txt");

#[derive(Debug, Parser)]
#[command(
    name = "ockam",
    term_width = 100,
    about = ABOUT,
    long_about = ABOUT,
    after_long_help = help::template(HELP_DETAIL),
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
    #[arg(hide = help::hide(), global = true, long)]
    no_color: bool,

    /// Output format
    #[arg(
        hide = help::hide(),
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

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Clone)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub config: OckamConfig,
    pub state: CliState,
}

impl CommandGlobalOpts {
    fn new(global_args: GlobalArgs, config: OckamConfig) -> Self {
        Self {
            global_args,
            config,
            state: CliState::new().expect("Failed to load CLI state"),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum OckamSubcommand {
    #[command(display_order = 800)]
    Enroll(EnrollCommand),
    #[command(display_order = 801)]
    Space(SpaceCommand),
    #[command(display_order = 802)]
    Project(ProjectCommand),
    #[command(display_order = 803)]
    Status(StatusCommand),
    #[command(display_order = 804)]
    Reset(ResetCommand),

    #[command(display_order = 811)]
    Node(NodeCommand),
    #[command(display_order = 812)]
    Identity(IdentityCommand),
    #[command(display_order = 813)]
    TcpListener(TcpListenerCommand),
    #[command(display_order = 814)]
    TcpConnection(TcpConnectionCommand),
    #[command(display_order = 815)]
    TcpOutlet(TcpOutletCommand),
    #[command(display_order = 816)]
    TcpInlet(TcpInletCommand),
    #[command(display_order = 817)]
    SecureChannelListener(SecureChannelListenerCommand),
    #[command(display_order = 818)]
    SecureChannel(SecureChannelCommand),
    #[command(display_order = 819)]
    Forwarder(ForwarderCommand),
    #[command(display_order = 820)]
    Message(MessageCommand),
    #[command(display_order = 821)]
    Policy(PolicyCommand),

    #[command(display_order = 900)]
    Completion(CompletionCommand),

    Authenticated(AuthenticatedCommand),
    Configuration(ConfigurationCommand),
    Credential(CredentialCommand),
    Service(ServiceCommand),
    Vault(VaultCommand),
    Subscription(SubscriptionCommand),
    Admin(AdminCommand),
    Manpages(ManpagesCommand),
    Lease(LeaseCommand),
}

pub fn run() {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();
    let command: OckamCommand = OckamCommand::parse_from(input);

    if !command.global_args.test_argument_parser {
        check_if_an_upgrade_is_available();
    }

    if !command.global_args.quiet {
        setup_logging(command.global_args.verbose, command.global_args.no_color);
        tracing::debug!("{}", Version::short());
        tracing::debug!("Parsed {:?}", &command);
    }

    command.run();
}

impl OckamCommand {
    pub fn run(self) {
        let config = OckamConfig::load().expect("Failed to load config");
        let options = CommandGlobalOpts::new(self.global_args, config);

        // If test_argument_parser is true, command arguments are checked
        // but the command is not executed. This is useful to test arguments
        // without having to execute their logic.
        if options.global_args.test_argument_parser {
            return;
        }

        match self.subcommand {
            OckamSubcommand::Enroll(c) => c.run(options),
            OckamSubcommand::Space(c) => c.run(options),
            OckamSubcommand::Project(c) => c.run(options),
            OckamSubcommand::Status(c) => c.run(options),
            OckamSubcommand::Reset(c) => c.run(options),

            OckamSubcommand::Node(c) => c.run(options),
            OckamSubcommand::Identity(c) => c.run(options),
            OckamSubcommand::TcpListener(c) => c.run(options),
            OckamSubcommand::TcpConnection(c) => c.run(options),
            OckamSubcommand::TcpOutlet(c) => c.run(options),
            OckamSubcommand::TcpInlet(c) => c.run(options),
            OckamSubcommand::SecureChannelListener(c) => c.run(options),
            OckamSubcommand::SecureChannel(c) => c.run(options),
            OckamSubcommand::Forwarder(c) => c.run(options),
            OckamSubcommand::Message(c) => c.run(options),
            OckamSubcommand::Policy(c) => c.run(options),

            OckamSubcommand::Completion(c) => c.run(),

            OckamSubcommand::Authenticated(c) => c.run(),
            OckamSubcommand::Configuration(c) => c.run(options),
            OckamSubcommand::Credential(c) => c.run(options),
            OckamSubcommand::Service(c) => c.run(options),
            OckamSubcommand::Vault(c) => c.run(options),
            OckamSubcommand::Subscription(c) => c.run(options),
            OckamSubcommand::Admin(c) => c.run(options),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Lease(c) => c.run(options),
        }
    }
}

fn replace_hyphen_with_stdin(s: String) -> String {
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
