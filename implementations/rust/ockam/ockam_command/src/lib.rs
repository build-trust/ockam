//! Orchestrate end-to-end encryption, mutual authentication, key management,
//! credential management, and authorization policy enforcement — at scale.

mod authenticated;
mod completion;
mod configuration;
mod credential;
mod enroll;
mod error;
mod forwarder;
mod help;
mod identity;
mod message;
mod node;
mod project;
mod secure_channel;
mod service;
mod space;
mod tcp;
mod terminal;
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
use message::MessageCommand;
use node::NodeCommand;
use project::ProjectCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
use space::SpaceCommand;
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use util::{exitcode, exitcode::ExitCode, setup_logging, OckamConfig};
use vault::VaultCommand;
use version::Version;

use clap::{ArgEnum, Args, Parser, Subcommand};

const ABOUT: &str = "\
Orchestrate end-to-end encryption, mutual authentication, key management,
credential management, and authorization policy enforcement — at scale.
";

const HELP_DETAIL: &str = "\
BACKGROUND:
    Modern applications are distributed and have an unwieldy number of
    interconnections that must trustfully exchange data. Ockam makes it simple
    to build secure by-design applications that have granular control over every
    trust and access decision.

EXAMPLES:

```sh
    # Create three local Ockam nodes n1, n2 & n3
    $ for i in {1..3}; do ockam node create \"n$i\"; done

    # Create a mutually authenticated, authorized, end-to-end encrypted secure channel
    # and send an end-to-end encrypted message through it.
    $ ockam secure-channel create --from n1 --to /node/n2/node/n3/service/api \\
         | ockam message send \"hello ockam\" --from n1 --to -/service/uppercase
    HELLO OCKAM
```
";

#[derive(Debug, Parser)]
#[clap(
    name = "ockam",
    term_width = 100,
    about = ABOUT,
    long_about = ABOUT,
    help_template = help::template(HELP_DETAIL),
    version,
    long_version = Version::long(),
    propagate_version(true),
)]
pub struct OckamCommand {
    #[clap(subcommand)]
    subcommand: OckamSubcommand,

    #[clap(flatten)]
    global_args: GlobalArgs,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Do not print any informational or trace messages.
    #[clap(global = true, long, short, conflicts_with("verbose"))]
    quiet: bool,

    /// Increase verbosity of output.
    #[clap(
        global = true,
        long,
        short,
        conflicts_with("quiet"),
        parse(from_occurrences)
    )]
    verbose: u8,

    /// Output without any colors.
    #[clap(global = true, long, action, hide = help::hide())]
    no_color: bool,

    ///
    #[clap(global = true, long = "output", value_enum, default_value = "plain", hide = help::hide())]
    output_format: OutputFormat,

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[clap(global = true, long, hide = true)]
    test_argument_parser: bool,
}

#[derive(Debug, Clone, ArgEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Clone)]
pub struct CommandGlobalOpts {
    pub global_args: GlobalArgs,
    pub config: OckamConfig,
}

impl CommandGlobalOpts {
    fn new(global_args: GlobalArgs, config: OckamConfig) -> Self {
        Self {
            global_args,
            config,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum OckamSubcommand {
    #[clap(display_order = 800)]
    Enroll(EnrollCommand),
    #[clap(display_order = 801)]
    Space(SpaceCommand),
    #[clap(display_order = 802)]
    Project(ProjectCommand),

    #[clap(display_order = 811)]
    Node(NodeCommand),
    #[clap(display_order = 812)]
    Identity(IdentityCommand),
    #[clap(display_order = 813)]
    TcpListener(TcpListenerCommand),
    #[clap(display_order = 814)]
    TcpConnection(TcpConnectionCommand),
    #[clap(display_order = 815)]
    TcpOutlet(TcpOutletCommand),
    #[clap(display_order = 816)]
    TcpInlet(TcpInletCommand),
    #[clap(display_order = 817)]
    SecureChannelListener(SecureChannelListenerCommand),
    #[clap(display_order = 818)]
    SecureChannel(SecureChannelCommand),
    #[clap(display_order = 819)]
    Forwarder(ForwarderCommand),
    #[clap(display_order = 820)]
    Message(MessageCommand),

    #[clap(display_order = 900)]
    Completion(CompletionCommand),

    Authenticated(AuthenticatedCommand),
    Configuration(ConfigurationCommand),
    Credential(CredentialCommand),
    Service(ServiceCommand),
    Vault(VaultCommand),
}

pub fn run() {
    let input = std::env::args().map(replace_hyphen_with_stdin);

    let command: OckamCommand = OckamCommand::parse_from(input);
    let config = OckamConfig::load();

    if !command.global_args.quiet {
        setup_logging(command.global_args.verbose, command.global_args.no_color);
        tracing::debug!("Parsed {:?}", &command);
    }

    let options = CommandGlobalOpts::new(command.global_args, config);

    // If test_argument_parser is true, command arguments are checked
    // but the command is not executed. This is useful to test arguments
    // without having to execute their logic.
    if options.global_args.test_argument_parser {
        return;
    }

    // FIXME
    let _verbose = options.global_args.verbose;

    match command.subcommand {
        OckamSubcommand::Authenticated(c) => c.run(),
        OckamSubcommand::Configuration(c) => c.run(options),
        OckamSubcommand::Enroll(c) => c.run(options),
        OckamSubcommand::Forwarder(c) => c.run(options),
        OckamSubcommand::Message(c) => c.run(options),
        OckamSubcommand::Node(c) => c.run(options),
        OckamSubcommand::Project(c) => c.run(options),
        OckamSubcommand::Space(c) => c.run(options),
        OckamSubcommand::TcpConnection(c) => c.run(options),
        OckamSubcommand::TcpInlet(c) => c.run(options),
        OckamSubcommand::TcpListener(c) => c.run(options),
        OckamSubcommand::TcpOutlet(c) => c.run(options),
        OckamSubcommand::Vault(c) => c.run(options),
        OckamSubcommand::Identity(c) => c.run(options),
        OckamSubcommand::SecureChannel(c) => c.run(options),
        OckamSubcommand::SecureChannelListener(c) => c.run(options),
        OckamSubcommand::Service(c) => c.run(options),
        OckamSubcommand::Completion(c) => c.run(),
        OckamSubcommand::Credential(c) => c.run(options),
    }
}

fn replace_hyphen_with_stdin(s: String) -> String {
    use std::io;
    if s.contains("/-") {
        let mut buffer = String::new();
        io::stdin()
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
        io::stdin()
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
