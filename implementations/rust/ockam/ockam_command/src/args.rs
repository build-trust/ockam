use clap::{Args, Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
#[clap(name = "ockam", version)]
pub struct CliArgs {
    #[clap(subcommand)]
    pub command: Command,
    /// Increase verbosity of logging output.
    ///
    ///   `-v` => Info level, and output extra debug information.
    ///   `-vv` => Debug level.
    ///   `-vvv` => Trace level.
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Args)]
pub struct InletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Bind address for the inlet to listen on.
    pub inlet_address: String,
}

#[derive(Clone, Debug, Subcommand)]
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
    /// This is equivalent to adding the identifier to the end of the the list
    /// in `<ockam_dir>/trusted` (`~/.config/ockam/trusted` by default, but
    /// code that `$OCKAM_DIR/trusted` if overwritten).
    #[clap(display_order = 1003)]
    AddTrustedIdentity(AddTrustedIdentityOpts),
    /// Print the identifier for the currently configured identity.
    #[clap(display_order = 1004)]
    PrintIdentity,
    /// Print path to the ockam directory.
    ///
    /// This is usually `$OCKAM_DIR` or `~/.config/ockam`, but in some cases can
    /// be different, such as on Windows, unixes where `$XDG_CONFIG_HOME` has
    /// been modified, etc.
    #[clap(display_order = 1005)]
    PrintPath,
    StartNode(StartNodeOpts),
    #[clap(subcommand)]
    Api(Api),
}

#[derive(Clone, Debug, Subcommand)]
pub enum Api {
    #[clap(subcommand)]
    Nodes(Nodes),
}

#[derive(Clone, Debug, Args)]
pub struct OutletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Address of tcp service running on Control Plane node that will receive
    /// connections from the Outlet
    pub outlet_target: String,
}

#[derive(Clone, Debug, Args)]
pub struct IdentityOpts {
    /// If an ockam identity already exists, overwrite it.
    ///
    /// This is a destructive operation and cannot be undone.
    ///
    /// Note: This only applies to the `<ockam_dir>/identity.json` files,
    /// and not to `<ockam_dir>/trusted`, which is left as-is must be managed manually.
    /// For example, with the `ockam add-trusted-identity` subcommand)
    #[clap(long)]
    pub overwrite: bool,
}

#[derive(Clone, Debug, Args)]
pub struct AddTrustedIdentityOpts {
    /// Discard any identities currently in `~/.config/ockam/trusted`, and
    /// replace them with the ones provided by this command.
    #[clap(long)]
    pub only: bool,
    /// The identity to trust, or space/comma-separated list of identities.
    ///
    /// Some effort is taken to avoid writing the file when not necessary, and
    /// to avoid adding duplicates entries in the file. Note that
    pub to_trust: String,
}

#[derive(Clone, Debug, Args)]
pub struct StartNodeOpts {
    #[clap(long, default_value = "127.0.0.1:62526")]
    pub listen_addr: String,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Nodes {
    Create {
        #[clap(long)]
        addr: String,
        #[clap(long)]
        name: String,
    },
    Get {
        #[clap(long)]
        addr: String,
        #[clap(long)]
        id: String,
    },
    List {
        #[clap(long)]
        addr: String,
    },
    Delete {
        #[clap(long)]
        addr: String,
        #[clap(long)]
        id: String,
    },
}
