//! This library is used by the `ockam` CLI (in `./bin/ockam.rs`).

mod cloud;
mod message;
mod node;
mod util;

use clap::{Parser, Subcommand};
use cloud::CloudCommand;
use message::MessageCommand;
use node::NodeCommand;
use util::setup_logging;

mod old;
use crate::util::embedded_node;
use old::cmd::identity::IdentityOpts;
use old::cmd::inlet::InletOpts;
use old::cmd::outlet::OutletOpts;
use old::AddTrustedIdentityOpts;
use old::{add_trusted, exit_with_result, node_subcommand, print_identity, print_ockam_dir};

#[derive(Clone, Debug, Parser)]
#[clap(name = "ockam", version)]
pub struct OckamCommand {
    #[clap(subcommand)]
    pub subcommand: OckamSubcommand,

    /// Increase verbosity of logging output.
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,

    /// Parse command's arguments without running it.
    /// Useful for testing purposes.
    #[clap(display_order = 1100, long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Subcommand)]
pub enum OckamSubcommand {
    /// Manage nodes
    #[clap(display_order = 900)]
    Node(NodeCommand),

    /// Send and receive messages
    #[clap(display_order = 901)]
    Message(MessageCommand),

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

    /// Cloud subcommands.
    #[clap(display_order = 1010)]
    Cloud(CloudCommand),
}

pub fn run() {
    let ockam_command = OckamCommand::parse();

    let verbose = ockam_command.verbose;
    setup_logging(verbose);

    tracing::debug!("Parsed {:?}", ockam_command);

    // Command arguments are checked but the command is not executed.
    // This is useful to test arguments without having to execute their logic.
    if ockam_command.dry_run {
        return;
    }

    match ockam_command.subcommand {
        OckamSubcommand::Node(command) => NodeCommand::run(command),
        OckamSubcommand::Message(command) => MessageCommand::run(command),
        OckamSubcommand::Cloud(command) => embedded_node(CloudCommand::run, command),

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
