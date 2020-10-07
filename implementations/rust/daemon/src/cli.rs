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
        help = r#"Data source providing input to `ockamd`, either "stdin" or a valid URI"#
    )]
    input: InputKind,

    /// Defines the kind of output where a message should be sent.
    #[structopt(
        long,
        default_value = "stdout",
        help = r#"Route to channel responder, e.g. udp://host:port[,udp://host:port],channel_address (note comma-separation) or "stdout""#
    )]
    output: OutputKind,

    #[structopt(
        long,
        default_value = DEFAULT_LOCAL_SOCKET,
        help = "Local node address and port to bind"
    )]
    local_socket: SocketAddr,

    /// Determine if data written to `output` should be decrypted.
    #[structopt(long, help = "Optionally decrypt messages to output")]
    decrypt_output: bool,

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
        help = r#"Start `ockamd` as an "initiator" or a "responder" of a secure channel"#
    )]
    role: ChannelRole,

    /// Define the channel responder address, currently obtained from a responder node.
    #[structopt(
        long,
        required_if("role", "initiator"),
        required_if("role", "init"),
        help = r#"Address used to reach channel "responder" on remote machine"#
    )]
    channel_responder_address: Option<String>,

    /// Define the worker address, currently obtained from a responder node.
    #[structopt(
        long,
        required_if("role", "initiator"),
        required_if("role", "init"),
        help = r#"Address used to reach "worker" on remote machine"#
    )]
    worker_address: Option<String>,

    /// Define which private key to use as the initiator's identity.
    #[structopt(
        long,
        required_if("role", "initiator"),
        required_if("role", "init"),
        help = "Name of the private key to use for the identity of the channel initiator"
    )]
    identity_name: Option<String>,

    /// Define the public key needed to communicate with the channel responder.
    #[structopt(
        long,
        required_if("role", "initiator"),
        required_if("role", "init"),
        help = "The public key provided by channel responder"
    )]
    responder_public_key: Option<String>,

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
            output: OutputKind::Stdout,
            local_socket: SocketAddr::from_str(DEFAULT_LOCAL_SOCKET)
                .expect("bad default set for local socket"),
            decrypt_output: true,
            vault: VaultKind::Filesystem,
            vault_path: PathBuf::from("ockamd_vault"),
            role: ChannelRole::Responder,
            channel_responder_address: None,
            worker_address: None,
            identity_name: None,
            responder_public_key: None,
        }
    }
}

impl Args {
    /// Parse the command line options into the Args struct.
    pub fn parse() -> Args {
        let mut args = Args::from_args();

        // validate provided arguments & override possibly fallible options
        match args.output_kind() {
            OutputKind::Channel(_) => {
                // disallow output to be decrypted if it's to be sent over a secure channel
                if args.decrypt_output {
                    args.decrypt_output = false;
                }
            }
            _ => {}
        }

        args
    }

    /// Checks which mode the executable was run in: Control or Server.
    pub fn exec_mode(&self) -> Mode {
        match self.control {
            true => Mode::Control,
            false => Mode::Server,
        }
    }

    pub fn role(&self) -> ChannelRole {
        self.role.clone()
    }

    pub fn output_kind(&self) -> OutputKind {
        self.output.clone()
    }

    pub fn local_socket(&self) -> SocketAddr {
        self.local_socket
    }

    pub fn decrypt_output(&self) -> bool {
        self.decrypt_output
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
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
#[derive(Clone, Copy, Debug, StructOpt, PartialEq)]
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
pub enum InputKind {
    Stdin,
    Channel(Route),
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
        if s == "stdout" {
            return Ok(OutputKind::Stdout);
        }

        let mut ret = Ok(OutputKind::Stdout);
        let mut route = Route { addresses: vec![] };

        s.split(',').for_each(|part| {
            match Url::parse(part) {
                Ok(u) => {
                    if !u.has_host() || u.port().is_none() {
                        ret = Err(format!("invalid URI: {}", part));
                    }

                    // TODO: add helper fn in message crate that peforms a FromStr and delegates to
                    // RouterAddress::* fn's if url scheme is udp, tcp, etc.
                    let addr = u.as_str().trim_start_matches("udp://");

                    if let Ok(router_addr) = RouterAddress::udp_router_address_from_str(addr) {
                        route.addresses.push(router_addr);
                    }
                }
                Err(_) => {
                    // try to get a channel address if the URI is not able to be parsed
                    match RouterAddress::channel_router_address_from_str(part) {
                        Ok(chan_addr) => {
                            route.addresses.push(chan_addr);
                        }
                        Err(e) => {
                            ret = Err(format!(
                                "failed to convert channel address from string: {:?}",
                                e
                            ));
                        }
                    }
                }
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

    if let Ok(output_kind) = OutputKind::from_str(
        "udp://10.10.1.3:9999,udp://192.168.33.4:4444,udp://10.2.22.2:22222".into(),
    ) {
        match output_kind {
            OutputKind::Channel(route) => {
                assert_eq!(route.addresses.len(), 3);
                route.addresses.iter().for_each(|addr| {
                    assert_eq!(addr.a_type, AddressType::Udp);
                })
            }
            _ => {}
        }
    }

    if let Ok(output_kind) =
        OutputKind::from_str("udp://117.2.34.1:11199,udp://10.2.34.3:8000,65ffa6cf".into())
    {
        match output_kind {
            OutputKind::Channel(route) => {
                assert_eq!(route.addresses.len(), 3);
                match route.addresses.last() {
                    Some(addr) => {
                        assert_eq!(addr.a_type, AddressType::Channel);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
