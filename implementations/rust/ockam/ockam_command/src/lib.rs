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
mod manpages;
mod message;
mod node;
mod policy;
mod project;
mod reset;
mod secure_channel;
mod service;
mod space;
#[allow(unused)]
mod state;
mod subscription;
mod tcp;
mod terminal;
mod upgrade;
mod util;
mod vault;
mod version;

use anyhow::Context;
use authenticated::AuthenticatedCommand;
use completion::CompletionCommand;
use configuration::ConfigurationCommand;
use credential::CredentialCommand;
use enroll::EnrollCommand;
use error::{Error, Result};
use forwarder::ForwarderCommand;
use identity::IdentityCommand;
use manpages::ManpagesCommand;
use message::MessageCommand;
use node::NodeCommand;
use policy::PolicyCommand;
use project::ProjectCommand;
use reset::ResetCommand;
use secure_channel::{listener::SecureChannelListenerCommand, SecureChannelCommand};
use service::ServiceCommand;
use space::SpaceCommand;
use std::path::PathBuf;
use tcp::{
    connection::TcpConnectionCommand, inlet::TcpInletCommand, listener::TcpListenerCommand,
    outlet::TcpOutletCommand,
};
use util::{exitcode, exitcode::ExitCode, setup_logging, OckamConfig};
use vault::VaultCommand;
use version::Version;

use crate::admin::AdminCommand;
use crate::node::util::run::CommandSection;
use crate::state::CliState;
use crate::subscription::SubscriptionCommand;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use upgrade::check_if_an_upgrade_is_available;

const ABOUT: &str = "\
Orchestrate end-to-end encryption, mutual authentication, key management,
credential management, and authorization policy enforcement — at scale.
";

const HELP_DETAIL: &str = "\
About:
    Orchestrate end-to-end encryption, mutual authentication, key management,
    credential management, and authorization policy enforcement — at scale.

    Modern applications are distributed and have an unwieldy number of
    interconnections that must trustfully exchange data. Ockam makes it simple
    to build secure by-design applications that have granular control over every
    trust and access decision.

Examples:

    Let's walk through a simple example to create an end-to-end encrypted,
    mutually authenticated, secure and private cloud relay – for any application.

    First let's enroll with Ockam Orchestrator where we'll create a managed cloud
    based relay that will move end-to-end encrypted  data between distributed parts
    of our application.

```sh
    # Create a cryptographic identity and enroll with Ockam Orchestrator.
    # This will sign you up for an account with Ockam Orchestrator and setup a
    # hobby space and project for you.
    $ ockam enroll
```

    You can also create encrypted relays outside the orchestrator.
    See `ockam forwarder --help`.

    Application Service
    ------

    Next let's prepare the service side of our application.

```sh
    # Start our application service, listening on a local ip and port, that clients
    # would access through the cloud relay. We'll use a simple http server for our
    # first example but this could be some other application service.
    $ python3 -m http.server --bind 127.0.0.1 5000

    # Setup an ockam node, called blue, as a sidecar next to our application service.
    $ ockam node create blue

    # Create a tcp outlet on the blue node to send raw tcp traffic to the application service.
    $ ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000

    # Then create a forwarding relay at your default orchestrator project to blue.
    $ ockam forwarder create blue --at /project/default --to /node/blue
```

    Application Client
    ------

    Now on the client side:

```sh
    # Setup an ockam node, called green, as a sidecar next to our application service.
    $ ockam node create green

    # Then create an end-to-end encrypted secure channel with blue, through the cloud relay.
    # Then tunnel traffic from a local tcp inlet through this end-to-end secure channel.
    $ ockam secure-channel create --from /node/green \\
        --to /project/default/service/forward_to_blue/service/api \\
            | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

    # Access the application service though the end-to-end encrypted, secure relay.
    $ curl 127.0.0.1:7000
```

    We just created end-to-end encrypted, mutually authenticated, and authorized
    secure communication between a tcp client and server. This client and server
    can be running in separate private networks / NATs. We didn't have to expose
    our server by opening a port on the Internet or punching a hole in our firewall.

    The two sides authenticated and authorized each other's known, cryptographically
    provable identifiers. In later examples we'll see how we can build granular,
    attribute-based access control with authorization policies.
";

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

    #[command(flatten)]
    export: ExportCommandArgs,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct ExportCommandArgs {
    /// Export the command input to a file.
    /// Used to run a set of commands after creating a node with `ockam node create --run commands.json`
    #[arg(global = true, long = "export", hide_short_help = true, hide = true)]
    export_path: Option<PathBuf>,

    /// Section of the config file to export the command to.
    #[arg(
        global = true,
        long = "export-section",
        hide_short_help = true,
        hide = true,
        value_enum
    )]
    section: Option<CommandSection>,

    /// Flag to indicate that the exported command should pipe its output.
    #[arg(global = true, long, hide_short_help = true, hide = true, action = ArgAction::SetTrue)]
    pipe: Option<bool>,
}

impl ExportCommandArgs {
    pub fn pipe(&self) -> bool {
        self.pipe.unwrap_or(false)
    }
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

    #[command(display_order = 900)]
    EnableAwsKms,

    Authenticated(AuthenticatedCommand),
    Configuration(ConfigurationCommand),
    Credential(CredentialCommand),
    Service(ServiceCommand),
    Vault(VaultCommand),
    Subscription(SubscriptionCommand),
    Admin(AdminCommand),
    Manpages(ManpagesCommand),
}

pub fn run() {
    let input = std::env::args()
        .map(replace_hyphen_with_stdin)
        .collect::<Vec<_>>();
    let args = input.clone();
    let command: OckamCommand = OckamCommand::parse_from(input);

    if !command.global_args.test_argument_parser {
        check_if_an_upgrade_is_available();
    }

    if !command.global_args.quiet {
        setup_logging(command.global_args.verbose, command.global_args.no_color);
        tracing::debug!("{}", Version::short());
        tracing::debug!("Parsed {:?}", &command);
    }

    if let Some(path) = command.global_args.export.export_path {
        let section = command.global_args.export.section.unwrap_or_default();
        let pipe = command.global_args.export.pipe;
        node::util::run::CommandsRunner::export(path, section, args, pipe)
            .context("Failed to export command")
            .unwrap();
        return;
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
            OckamSubcommand::Authenticated(c) => c.run(),
            OckamSubcommand::Configuration(c) => c.run(options),
            OckamSubcommand::Enroll(c) => c.run(options),
            OckamSubcommand::Forwarder(c) => c.run(options),
            OckamSubcommand::Manpages(c) => c.run(),
            OckamSubcommand::Message(c) => c.run(options),
            OckamSubcommand::Node(c) => c.run(options),
            OckamSubcommand::Policy(c) => c.run(options),
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
            OckamSubcommand::Subscription(c) => c.run(options),
            OckamSubcommand::Reset(c) => c.run(options),
            OckamSubcommand::Admin(c) => c.run(options),
            OckamSubcommand::EnableAwsKms => {
                options.config.enable_aws_kms(true);
                if let Err(e) = options.config.persist_config_updates() {
                    eprintln!("Failed to persist config file: {:?}", anyhow::Error::from(e));
                    std::process::exit(exitcode::IOERR)
                }
            }
        }
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
