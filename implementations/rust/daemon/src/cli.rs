use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use ockam_message::message::{Route, RouterAddress};

use structopt::{clap::ArgSettings::Hidden, StructOpt};
use url::Url;

/// The port on which the config updater runs and accepts Config messages.
pub const DEFAULT_CONFIG_PORT: u16 = 11199;

const DEFAULT_LOCAL_SOCKET: &str = "127.0.0.1:0";

/// Command-line arguments passed to `ockamd`.
#[derive(StructOpt)]
#[structopt(
    author = "Ockam Developers (ockam.io)",
    about = "Encrypt, route, and decrypt messages using the Ockam daemon."
)]
pub struct Args {
    /// Defines the kind of input from which a message should be read.
    #[structopt(
        long,
        default_value = "stdin",
        help = "Data source providing input to `ockamd`"
    )]
    input: InputKind,

    /// Defines the route where a message should be sent.
    #[structopt(
        long,
        default_value = "stdout",
        help = r#"Route to channel responder, e.g. udp://host:port[,udp://host:port] (note comma-separation) or "stdout""#
    )]
    route: OutputKind,

    #[structopt(
        long,
        default_value = DEFAULT_LOCAL_SOCKET,
        help = "Local node address and port to bind"
    )]
    local_socket: SocketAddr,

    /// Defines the kind of Ockam vault implementation to use.
    #[structopt(
        long,
        default_value = "FILESYSTEM",
        help = "Specify which type of Ockam vault to use for this instance of `ockamd`"
    )]
    vault: VaultKind,

    /// Path on disk where the vault data is stored (used with the FILESYSTEM vault).
    #[structopt(
        parse(from_os_str),
        long,
        default_value = "ockamd_vault",
        required_if("vault", "FILESYSTEM"),
        help = "Filepath on disk to pre-existing private keys to be used by the filesystem vault"
    )]
    vault_path: PathBuf,

    /// Start the `ockamd` process as the initiator or responder of a secure channel.
    #[structopt(
        long,
        default_value = "initiator",
        help = r#"Start `ockamd` as an "initiator" or a "responder" of a secure channel"#
    )]
    role: ChannelRole,

    /// Define which private key to use as the initiator's identity.
    #[structopt(
        long,
        help = "Name of the private key to use for the identity of the channel initiator"
    )]
    identity_name: Option<String>,

    /// Define the public key provided by the remote service.
    #[structopt(
        long,
        required_if("role", "initiator"),
        required_if("role", "init"),
        help = "The public key provided by the remote service"
    )]
    service_public_key: Option<String>,

    #[structopt(long,
    // required_if("role", "initiator"),
    // required_if("role", "init"),
    help = "Address used to reach the service on remote machine")]
    service_address: Option<String>,

    // TODO: expose `control` and `control_port` once runtime configuration is needed.
    #[structopt(
        short,
        long,
        help = "Execute `ockamd` in control-mode, otherwise will start as a long-running process",
        set = Hidden,
    )]
    control: bool,
    #[structopt(
        short = "p",
        long = "port",
        default_value = "11199",
        help = "Port for runtime configuration updates",
        set = Hidden,
    )]
    control_port: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            control: false,
            control_port: DEFAULT_CONFIG_PORT,
            input: InputKind::Stdin,
            route: OutputKind::Stdout,
            local_socket: SocketAddr::from_str(DEFAULT_LOCAL_SOCKET)
                .expect("bad default set for local socket"),
            vault: VaultKind::Filesystem,
            vault_path: PathBuf::from("ockamd_vault"),
            role: ChannelRole::Responder,
            service_address: Some("01020304".into()),
            identity_name: None,
            service_public_key: None,
        }
    }
}

impl Args {
    /// Parse the command line options into the Args struct.
    pub fn parse() -> Args {
        // validate provided arguments & override possibly fallible options
        // TODO: what should be disallowed that the CLI validation wont handle?
        Args::from_args()
    }

    /// Checks which mode the executable was run in: Control or Server.
    pub fn exec_mode(&self) -> Mode {
        match self.control {
            true => Mode::Control,
            false => Mode::Server,
        }
    }

    pub fn role(&self) -> ChannelRole {
        self.role
    }

    pub fn output_kind(&self) -> OutputKind {
        self.route.clone()
    }

    pub fn input_kind(&self) -> InputKind {
        self.input.clone()
    }

    pub fn local_socket(&self) -> SocketAddr {
        self.local_socket
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }

    pub fn service_public_key(&self) -> Option<String> {
        self.service_public_key.clone()
    }

    pub fn service_address(&self) -> Option<String> {
        self.service_address.clone()
    }
}

/// Specifies the implementation of a Ockam vault to be used.
pub enum VaultKind {
    Filesystem,
}

impl FromStr for VaultKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FILESYSTEM" => Ok(VaultKind::Filesystem),
            _ => Err("currently, 'FILESYSTEM' is the only supported vault option".into()),
        }
    }
}

/// Specifies which end of the secure channel the instance of `ockamd` is prepared to run in.
#[derive(Clone, Copy, Debug, StructOpt)]
pub enum ChannelRole {
    /// The Initiator role expects a channel responder address and a public key to use in order to
    /// communicate with the Responder end of the channel.
    Initiator,
    /// The Responder role will create a channel responder, and will instruct the program to print
    /// the responder's channel responder address and the public key it's advertising.
    Responder,
}

impl FromStr for ChannelRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "initiator" | "init" => Ok(ChannelRole::Initiator),
            "responder" | "resp" => Ok(ChannelRole::Responder),
            _ => Err("role must be set to either 'initiator' or 'responder'".into()),
        }
    }
}

/// The mode in which `ockamd` is to be run.
#[derive(Clone, Copy, Debug, StructOpt)]
pub enum Mode {
    /// Used for controlling configuration options at runtime, requiring that a
    /// Server process of `ockamd` is running.
    Control,
    /// Used to create a long-running process, to be executed with a particular
    /// input, e.g. Stdin.
    Server,
}

/// Specifies where input to `ockamd` should be read.
#[derive(Clone)]
pub enum InputKind {
    Stdin,
}

impl FromStr for InputKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdin" => Ok(InputKind::Stdin),
            _ => Err("currently, only 'stdin' is a supported input type".into()),
        }
    }
}

/// Specifies where ouput from `ockamd` should be written.
#[derive(Clone)]
pub enum OutputKind {
    Stdout,
    Channel(Route),
}

impl FromStr for OutputKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ret = Ok(OutputKind::Stdout);

        if s == "stdout" {
            return ret;
        }

        let mut route = Route { addresses: vec![] };

        s.split(',').for_each(|part| {
            match Url::parse(part) {
                Ok(u) => {
                    if !u.has_host() {
                        ret = Err(format!("invalid URI: {}", part));
                    }

                    // TODO: add helper fn in message crate that peforms a FromStr and delegates to
                    // RouterAddress::* fn's if url scheme is udp, tcp, etc.
                    let addr = u.as_str().trim().trim_start_matches("udp://");

                    if let Ok(router_addr) = RouterAddress::udp_router_address_from_str(addr) {
                        route.addresses.push(router_addr);
                    }
                }
                Err(e) => ret = Err(format!("failed to parse url: {:?}", e)),
            }
        });

        if !route.addresses.is_empty() && ret.is_ok() {
            ret = Ok(OutputKind::Channel(route))
        }

        ret
    }
}

#[test]
fn test_cli_args_output() {
    use ockam_message::message::AddressType;

    if let Ok(output_kind) = OutputKind::from_str("udp://127.0.0.1:12345".into()) {
        match output_kind {
            OutputKind::Channel(route) => {
                assert_eq!(route.addresses.len(), 1);
            }
            _ => {}
        }
    }

    let test_cases = [
        // route
        "udp://10.10.1.3:9999,udp://192.168.33.4:4444,udp://10.2.22.2:22222",
        // number of hops in route
        "3",
        // etc..
        "udp://16.31.56.22, udp://ockam.network, udp://14.172.71.124, udp://44.178.238.169",
        "4",
    ];

    test_cases.windows(2).for_each(|route_hop| {
        if let Ok(output_kind) = OutputKind::from_str(route_hop[0]) {
            match output_kind {
                OutputKind::Channel(route) => {
                    assert_eq!(route.addresses.len(), route_hop[1].parse().unwrap());
                    route.addresses.iter().for_each(|addr| {
                        assert_eq!(addr.a_type, AddressType::Udp);
                    })
                }
                _ => {}
            }
        }
    });
}
