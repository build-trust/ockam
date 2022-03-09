use std::path::PathBuf;

use clap::{ArgEnum, Parser};

#[derive(Clone, Debug, Parser)]
#[clap(name = "ockam", version)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
    /// Increase verbosity of logging output.
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, clap::Args)]
pub struct InletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Bind address for the inlet to listen on.
    pub inlet_address: String,
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum Command {
    /// Start an outlet.
    #[clap(display_order = 1000)]
    CreateOutlet(OutletOpts),
    /// Start an inlet.
    #[clap(display_order = 1001)]
    CreateInlet(InletOpts),
    /// Create an ockam identity.
    #[clap(display_order = 1002)]
    CreateIdentity(IdentityOpts),
    /// Add an identity (or multiple) to the trusted list.
    ///
    /// This is equivalent to adding the identifier to the end of the
    /// the list in `~/.config/ockam/trusted` (or `$OCKAM_DIR/trusted`).
    #[clap(display_order = 1003)]
    AddTrustedIdentity(AddTrustedIdentityOpts),
    /// Print the identifier for the currently configured identity.
    #[clap(display_order = 1004)]
    PrintIdentity,
}

#[derive(Clone, Debug, clap::Args)]
pub struct OutletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Address of tcp service running on Control Plane node that will receive
    /// connections from the Outlet
    pub outlet_target: String,
}

#[derive(Clone, Debug, clap::Args)]
pub struct IdentityOpts {
    /// If an ockam identity already exists, overwrite it. This is a destructive
    /// operation and cannot be undone.
    #[clap(long)]
    pub overwrite: bool,
}

#[derive(Clone, Debug, clap::Args)]
pub struct AddTrustedIdentityOpts {
    /// Discard any identities currently in `~/.config/ockam/trusted`, and
    /// replace them with the ones provided by this command.
    #[clap(long)]
    pub only: bool,
    /// Identity to trust, or space/comma-separated list of identities.
    ///
    /// Multiple identities may be passed in, separated by whitespace or commas.
    pub to_trust: String,
}
