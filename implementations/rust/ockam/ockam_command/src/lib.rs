//! This library is used by the `ockam` CLI (in `./bin/ockam.rs`).

mod alias;
mod authenticated;
mod enroll;
mod forwarder;
mod identity;
mod message;
mod node;
mod portal;
mod project;
mod secure_channel;
mod secure_channel_listener;
mod service;
mod space;
mod tcp;
mod util;
mod vault;

use alias::AliasCommand;
use authenticated::AuthenticatedCommand;
use enroll::EnrollCommand;
use forwarder::ForwarderCommand;
use message::MessageCommand;
use node::NodeCommand;
use portal::PortalCommand;
use project::ProjectCommand;
use secure_channel::SecureChannelCommand;
use secure_channel_listener::SecureChannelListenerCommand;
use space::SpaceCommand;
use tcp::{connection::TcpConnectionCommand, listener::TcpListenerCommand};

// to be removed
mod old;

use old::cmd::identity::IdentityOpts;
use old::cmd::inlet::InletOpts;
use old::cmd::outlet::OutletOpts;
use old::AddTrustedIdentityOpts;
use old::{add_trusted, exit_with_result, node_subcommand, print_identity, print_ockam_dir};

use crate::enroll::GenerateEnrollmentTokenCommand;
use crate::identity::IdentityCommand;
use crate::service::ServiceCommand;
use crate::util::OckamConfig;
use crate::vault::VaultCommand;
use clap::{crate_version, ArgEnum, Args, ColorChoice, Parser, Subcommand};
use util::setup_logging;

const HELP_TEMPLATE: &str = "\
{before-help}
{name} {version} {author-with-newline}
{about-with-newline}
{usage-heading}
    {usage}

{all-args}

LEARN MORE
    Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
    Learn more at https://docs.ockam.io/get-started/command-line

FEEDBACK
    If you have any questions or feedback, please start a discussion
    on Github https://github.com/build-trust/ockam/discussions/new
";

fn long_version() -> &'static str {
    let crate_version = crate_version!();
    let git_hash = env!("GIT_HASH");
    let message = format!(
        "{}\n\nCompiled from (git hash): {}",
        crate_version, git_hash
    );

    Box::leak(message.into_boxed_str())
}

/// Work seamlessly with Ockam from the command line.
#[derive(Debug, Parser)]
#[clap(
    name = "ockam",
    version,
    long_version = long_version(),
    propagate_version(true),
    color(ColorChoice::Never),
    help_template = HELP_TEMPLATE,
)]
pub struct OckamCommand {
    #[clap(subcommand)]
    subcommand: OckamSubcommand,

    #[clap(flatten)]
    global_args: GlobalArgs,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Do not print trace messages.
    #[clap(global = true, long, short, conflicts_with("verbose"))]
    quiet: bool,

    /// Increase verbosity of trace messages.
    #[clap(
        global = true,
        long,
        short,
        conflicts_with("quiet"),
        parse(from_occurrences)
    )]
    verbose: u8,

    /// Disable ANSI terminal colors for trace messages.
    #[clap(global = true, long, action, hide = hide())]
    no_color: bool,

    #[clap(global = true, long = "format", value_enum, default_value = "plain", hide = hide())]
    output_format: OutputFormat,

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[clap(global = true, long, hide = true)]
    test_argument_parser: bool,
}

#[derive(Debug, Clone, ArgEnum)]
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
    /// Manage ockam node alias values
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Alias(AliasCommand),

    /// Send or receive messages
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Message(MessageCommand),

    /// Create, update or delete nodes
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Node(NodeCommand),

    /// Create, update, or delete tcp connections
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    TcpConnection(TcpConnectionCommand),

    /// Create, update, or delete tcp listeners
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    TcpListener(TcpListenerCommand),

    /// Manage Identities
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Identity(IdentityCommand),

    /// Manage Secure Channels
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    SecureChannel(SecureChannelCommand),

    /// Manage Secure Channel Listeners
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    SecureChannelListener(SecureChannelListenerCommand),

    // HIDDEN
    /// Manage authenticated attributes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Authenticated(AuthenticatedCommand),
    /// Enroll with Ockam Orchestrator
    // This command offers three different ways for enrolling depending on the information provided
    // by the user. The user can run the enrollment process by passing an `email` address, a `token`
    // previously generated by the `ockam token`, or using the Auth0 flow.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Enroll(EnrollCommand),

    /// Create forwarders
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Forwarder(ForwarderCommand),

    /// Generate an enrollment token
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, name = "token", hide = hide())]
    GenerateEnrollmentToken(GenerateEnrollmentTokenCommand),

    /// Create, update, or delete portal endpoints
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Portal(PortalCommand),

    /// Create, update or delete projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Project(ProjectCommand),

    /// Manage Services
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Service(ServiceCommand),

    /// Create, update or delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Space(SpaceCommand),

    /// Manage Vault
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Vault(VaultCommand),

    // OLD
    /// Start an outlet.
    #[clap(display_order = 1000, hide = true)]
    CreateOutlet(OutletOpts),

    /// Start an inlet.
    #[clap(display_order = 1001, hide = true)]
    CreateInlet(InletOpts),

    /// Create an ockam identity.
    #[clap(display_order = 1002, hide = true)]
    CreateIdentity(IdentityOpts),

    /// Add an identity (or multiple) to the trusted list.
    ///
    /// This is equivalent to adding the identifier to the end of the the list
    /// in `<ockam_dir>/trusted` (`~/.config/ockam/trusted` by default, but
    /// code that `$OCKAM_DIR/trusted` if overwritten).
    #[clap(display_order = 1003, hide = true)]
    AddTrustedIdentity(AddTrustedIdentityOpts),

    /// Print the identifier for the currently configured identity.
    #[clap(display_order = 1004, hide = true)]
    PrintIdentity,

    /// Print path to the ockam directory.
    ///
    /// This is usually `$OCKAM_DIR` or `~/.config/ockam`, but in some cases can
    /// be different, such as on Windows, unixes where `$XDG_CONFIG_HOME` has
    /// been modified, etc.
    #[clap(display_order = 1005, hide = true)]
    PrintPath,
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

fn hide() -> bool {
    match std::env::var("SHOW_HIDDEN") {
        Ok(v) => !v.eq_ignore_ascii_case("true"),
        Err(_e) => true,
    }
}

pub fn run() {
    let ockam_command: OckamCommand =
        OckamCommand::parse_from(std::env::args().map(replace_hyphen_with_stdin));
    let cfg = OckamConfig::load();

    if !ockam_command.global_args.quiet {
        setup_logging(
            ockam_command.global_args.verbose,
            ockam_command.global_args.no_color,
        );
        tracing::debug!("Parsed {:?}", &ockam_command);
    }

    let opts = CommandGlobalOpts::new(ockam_command.global_args, cfg);

    // If test_argument_parser is true, command arguments are checked
    // but the command is not executed. This is useful to test arguments
    // without having to execute their logic.
    if opts.global_args.test_argument_parser {
        return;
    }

    let verbose = opts.global_args.verbose;

    match ockam_command.subcommand {
        OckamSubcommand::Alias(command) => AliasCommand::run(opts, command),
        OckamSubcommand::Authenticated(command) => AuthenticatedCommand::run(command),
        OckamSubcommand::Enroll(command) => EnrollCommand::run(opts, command),
        OckamSubcommand::GenerateEnrollmentToken(command) => {
            GenerateEnrollmentTokenCommand::run(opts, command)
        }
        OckamSubcommand::Forwarder(command) => ForwarderCommand::run(opts, command),
        OckamSubcommand::Message(command) => MessageCommand::run(opts, command),
        OckamSubcommand::Node(command) => NodeCommand::run(opts, command),
        OckamSubcommand::Project(command) => ProjectCommand::run(opts, command),
        OckamSubcommand::Space(command) => SpaceCommand::run(opts, command),
        OckamSubcommand::TcpConnection(command) => TcpConnectionCommand::run(opts, command),
        OckamSubcommand::TcpListener(command) => TcpListenerCommand::run(opts, command),
        OckamSubcommand::Portal(command) => PortalCommand::run(opts, command),
        OckamSubcommand::Vault(command) => VaultCommand::run(opts, command),
        OckamSubcommand::Identity(command) => IdentityCommand::run(opts, command),
        OckamSubcommand::SecureChannel(command) => SecureChannelCommand::run(opts, command),
        OckamSubcommand::SecureChannelListener(command) => {
            SecureChannelListenerCommand::run(opts, command)
        }
        OckamSubcommand::Service(command) => ServiceCommand::run(opts, command),

        // OLD
        OckamSubcommand::CreateOutlet(arg) => {
            node_subcommand(verbose > 0, arg, old::cmd::outlet::run)
        }
        OckamSubcommand::CreateInlet(arg) => {
            node_subcommand(verbose > 0, arg, old::cmd::inlet::run)
        }
        OckamSubcommand::CreateIdentity(arg) => {
            node_subcommand(verbose > 0, arg, old::cmd::identity::run)
        }
        OckamSubcommand::AddTrustedIdentity(arg) => exit_with_result(verbose > 0, add_trusted(arg)),
        OckamSubcommand::PrintIdentity => node_subcommand(verbose > 0, (), print_identity),
        OckamSubcommand::PrintPath => exit_with_result(verbose > 0, print_ockam_dir()),
    }
}
