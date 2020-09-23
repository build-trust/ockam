use std::path::PathBuf;
use std::str::FromStr;

use ockam_message::message::Address;

use structopt::StructOpt;
use url::Url;

/// The port on which the config updater runs and accepts Config messages.
pub const DEFAULT_CONFIG_PORT: u16 = 11199;
/// Command-line arguments passed to `ockamd`.
#[derive(Debug, StructOpt)]
#[structopt(
    author = "Ockam Developers (ockam.io)",
    about = "Encrypt and route messages using the Ockam daemon."
)]
pub struct Args {
    #[structopt(
        short,
        long,
        help = "Execute `ockamd` in control-mode, otherwise will start as a long-running process"
    )]
    control: bool,
    #[structopt(
        short = "p",
        long = "port",
        default_value = "11199",
        help = "port for runtime configuration updates"
    )]
    control_port: u16,
    /// InputKind translates the provided argument from either a URI formatted string
    /// (e.g. udp://127.0.0.1:11199/abcdef) into a address usable for creating a secure channel, or
    /// the literal value "stdin" to instruct `ockamd` to use the STDIN handle to read input.
    #[structopt(
        short,
        long,
        default_value = "stdin",
        help = r#"data source providing input to `ockamd`, either "stdin" or a valid URI"#
    )]
    input: InputKind,
    /// OutputKind translates the provided argument from either a URI formatted string
    /// (e.g. udp://127.0.0.1:11199/abcdef) into a address usable for creating a secure channel, or
    /// the literal value "stdout" to instruct `ockamd` to use the STDOUT handle to write output.
    #[structopt(
        short,
        long,
        default_value = "stdout",
        help = "URI of remote Ockam node, e.g. udp://host:port[/channel_id]"
    )]
    output: OutputKind,
    /// Keys is the path on disk where the vault data is stored.
    #[structopt(
        parse(from_os_str),
        long,
        default_value = "ockamd_keys",
        help = "path on disk to pre-existing private keys"
    )]
    keys: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            control: false,
            control_port: DEFAULT_CONFIG_PORT,
            input: InputKind::Stdin,
            output: OutputKind::Stdout,
            keys: PathBuf::from("ockamd_keys"),
        }
    }
}

impl Args {
    /// Parse the command line options into the Args struct.
    pub fn parse() -> Args {
        Args::from_args()
    }

    /// Checks which mode the executable was run in: Control or Server.
    pub fn exec_mode(&self) -> Mode {
        match self.control {
            true => Mode::Control,
            false => Mode::Server,
        }
    }
}

/// The mode in which `ockamd` is to be run.
#[derive(Debug, StructOpt)]
pub enum Mode {
    /// Used for controlling configuration options at runtime, requiring that a
    /// Server process of `ockamd` is running.
    Control,
    /// Used to create a long-running process, to be executed with a particular
    /// input, e.g. Stdin.
    Server,
}

#[derive(Debug)]
pub enum InputKind {
    Stdin,
    Channel(Address),
}

impl FromStr for InputKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(u) => {
                return to_address(u).map(|addr| Self::Channel(addr));
            }
            Err(_e) => {
                if s == "stdin" {
                    return Ok(Self::Stdin);
                } else {
                    return Err(unrecognized_input("input", s));
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum OutputKind {
    Stdout,
    Channel(Address),
}

impl FromStr for OutputKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(u) => {
                return to_address(u).map(|addr| Self::Channel(addr));
            }
            Err(_e) => {
                if s == "stdout" {
                    return Ok(Self::Stdout);
                } else {
                    return Err(unrecognized_input("output", s));
                }
            }
        }
    }
}

fn to_address(u: Url) -> Result<Address, String> {
    if u.scheme() != "udp" {
        return Err("currently, UDP is the only supported transport. use a udp://host:port[/id] formatted URI.".into());
    }

    if u.host().is_none() || u.port().is_none() {
        return Err(format!("invalid address format: {}", u));
    }

    if let Ok(addr) = u.host().unwrap().to_string().parse() {
        Ok(Address::UdpAddress(addr, u.port().unwrap()))
    } else {
        Err(format!("failed to convert provided URI {} into Address", u).into())
    }
}

fn unrecognized_input(flag: &str, input: &str) -> String {
    format!("Unrecognized value ({:?}) for `{}` flag.", input, flag)
}
