use std::path::PathBuf;

use clap::Parser;

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
    CreateOutlet(OutletOpts),
    /// Start an inlet.
    CreateInlet(InletOpts),
    /// Create an ockam identity.
    CreateIdentity(IdentityOpts),
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
    /// If an ockam identity already exists, overwrite it.
    #[clap(long)]
    pub overwrite: bool,
}
