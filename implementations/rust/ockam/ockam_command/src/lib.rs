//! This library is used by the `ockam` CLI (in `./bin/ockam.rs`).

mod auth;
mod config;
mod enroll;
mod message;
mod node;
mod project;
mod space;
mod util;

use clap::{ColorChoice, Parser, Subcommand};

use auth::AuthCommand;
use enroll::EnrollCommand;
use message::MessageCommand;
use node::NodeCommand;
use project::ProjectCommand;
use space::SpaceCommand;
use util::setup_logging;

mod old;
use old::cmd::identity::IdentityOpts;
use old::cmd::inlet::InletOpts;
use old::cmd::outlet::OutletOpts;
use old::AddTrustedIdentityOpts;
use old::{add_trusted, exit_with_result, node_subcommand, print_identity, print_ockam_dir};

use crate::config::OckamConfig;

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

/// Work seamlessly with Ockam from the command line.
#[derive(Clone, Debug, Parser)]
#[clap(
    name = "ockam",
    version,
    propagate_version(true),
    color(ColorChoice::Never),
    help_template = HELP_TEMPLATE,
)]
pub struct OckamCommand {
    #[clap(subcommand)]
    subcommand: OckamSubcommand,

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

    // if test_argument_parser is true, command arguments are checked
    // but the command is not executed.
    #[clap(global = true, long, hide = true)]
    pub test_argument_parser: bool,

    /// A marker to indicate that this instance was spawned
    ///
    /// This is a quick work-around to avoid spamming the user with
    /// irrelevant log messages from embedded nodes, while letting
    /// spawned nodes log their full potential into their log files.
    #[clap(display_order = 1006, long, hide = true)]
    spawn_marker: bool,
}

#[derive(Clone, Debug, Subcommand)]
pub enum OckamSubcommand {
    /// Manage authenticated attributes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, subcommand)]
    Auth(AuthCommand),

    /// Enroll with Ockam Orchestrator
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Enroll(EnrollCommand),

    /// Send or receive messages
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Message(MessageCommand),

    /// Create, update or delete nodes
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Node(NodeCommand),

    /// Create, update or delete projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Project(ProjectCommand),

    /// Create, update or delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Space(SpaceCommand),

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

pub fn run() {
    let ockam_command = OckamCommand::parse();

    // If test_argument_parser is true, command arguments are checked
    // but the command is not executed. This is useful to test arguments
    // without having to execute their logic.
    if ockam_command.test_argument_parser {
        return;
    }

    let verbose = ockam_command.verbose;
    if ockam_command.spawn_marker {
        setup_logging(verbose);
        tracing::debug!("Parsed {:?}", ockam_command);
    }

    let mut cfg = OckamConfig::load();

    match ockam_command.subcommand {
        OckamSubcommand::Auth(command) => AuthCommand::run(command),
        OckamSubcommand::Enroll(command) => EnrollCommand::run(command),
        OckamSubcommand::Message(command) => MessageCommand::run(command),
        OckamSubcommand::Node(command) => NodeCommand::run(&mut cfg, command),
        OckamSubcommand::Project(command) => ProjectCommand::run(command),
        OckamSubcommand::Space(command) => SpaceCommand::run(command),

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
        OckamSubcommand::PrintIdentity => exit_with_result(verbose > 0, print_identity()),
        OckamSubcommand::PrintPath => exit_with_result(verbose > 0, print_ockam_dir()),
    }
}
