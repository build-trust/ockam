use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use structopt::StructOpt;

/// The port on which the config updater runs and accepts Config messages.
pub const DEFAULT_CONFIG_PORT: u16 = 11199;

/// Configuration options which are available to update during runtime via
/// running `ockamd` in control-mode.
pub struct Config {}

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
    #[structopt(
        short,
        long,
        default_value = "stdin",
        help = "data source providing input to `ockamd`"
    )]
    input: InputKind,
    #[structopt(
        short,
        long,
        help = "socket address of Ockam router, used when 'transport' option is set"
    )]
    output: Option<IpAddr>,
    #[structopt(
        short,
        long,
        default_value = "stdout",
        help = "transport over which encrypted messages are sent"
    )]
    transport: OutputKind,
    #[structopt(
        parse(from_os_str),
        long,
        default_value = "ockamd.pub",
        help = "path on disk to pre-existing public key"
    )]
    public_key: PathBuf,
}

impl FromStr for InputKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdin" => Ok(InputKind::Stdin),
            _ => Err(unrecognized_input("input", s)),
        }
    }
}

impl FromStr for OutputKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdout" => Ok(OutputKind::Stdout),
            "udp" => Ok(OutputKind::Udp),
            _ => Err(unrecognized_input("transport", s)),
        }
    }
}

fn unrecognized_input(flag: &str, input: &str) -> String {
    format!("Unrecognized value ({:?}) for `{}` flag.", input, flag)
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

#[derive(Debug, StructOpt)]
pub enum InputKind {
    Stdin,
}

#[derive(Debug, StructOpt)]
pub enum OutputKind {
    Stdout,
    Udp,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            control: false,
            control_port: DEFAULT_CONFIG_PORT,
            input: InputKind::Stdin,
            output: None, // TODO: should it be more literally named & exposed as such to the user, e.g. "onward_route"?
            transport: OutputKind::Stdout,
            public_key: PathBuf::from("ockamd.pub"),
        }
    }
}

impl Args {
    pub fn parse() -> Args {
        Args::from_args()
    }
}
