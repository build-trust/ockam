use std::net::SocketAddr;
use std::path::PathBuf;

use crate::cli;

use ockam_message::message::{Address, Route};

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Initiator,
    Responder,
    Router,
}

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Stdin,
}

#[derive(Debug, Clone)]
pub enum AddonKind {
    InfluxDb(url::Url, String),
}

#[derive(Debug, Clone)]
pub struct Config {
    onward_route: Option<Route>,
    output_to_stdout: bool,
    local_socket: SocketAddr,
    router_socket: Option<SocketAddr>,
    channel_to_sink: Option<String>,
    role: Role,
    vault_path: PathBuf,
    input_kind: Input,
    remote_public_key: Option<String>,
    service_address: Option<String>,
    identity_name: String,
    addon: Option<AddonKind>,
}

impl Default for Config {
    fn default() -> Self {
        cli::Args::default().into()
    }
}

impl Config {
    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }

    pub fn onward_route(&self) -> Option<Route> {
        self.onward_route.clone()
    }

    pub fn input_kind(&self) -> Input {
        self.input_kind
    }

    pub fn local_socket(&self) -> SocketAddr {
        self.local_socket
    }

    pub fn router_socket(&self) -> Option<SocketAddr> {
        self.router_socket
    }

    pub fn remote_public_key(&self) -> Option<String> {
        self.remote_public_key.clone()
    }

    pub fn channel_to_sink(&self) -> Option<String> {
        self.channel_to_sink.clone()
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn service_address(&self) -> Option<String> {
        self.service_address.clone()
    }

    pub fn identity_name(&self) -> String {
        self.identity_name.clone()
    }

    pub fn addon(&self) -> Option<AddonKind> {
        self.addon.clone()
    }
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            onward_route: None,
            output_to_stdout: false,
            local_socket: args.local_socket(),
            channel_to_sink: args.channel_to_sink(),
            router_socket: args.router_socket(),
            role: Role::Initiator,
            vault_path: args.vault_path(),
            input_kind: Input::Stdin,
            remote_public_key: args.service_public_key(),
            service_address: args.service_address(),
            identity_name: args.identity_name(),
            addon: if let Some(a) = args.addon() {
                match a {
                    cli::Addon::InfluxDb(u, db) => Some(AddonKind::InfluxDb(u, db)),
                }
            } else {
                None
            },
        };

        match args.output_kind() {
            cli::OutputKind::Channel(route) => {
                cfg.onward_route = Some(route);
            }
            cli::OutputKind::Stdout => {
                cfg.output_to_stdout = true;
            }
        }

        cfg.role = match args.role() {
            cli::ChannelRole::Source => Role::Initiator,
            cli::ChannelRole::Sink => Role::Responder,
            cli::ChannelRole::Router => Role::Router,
        };

        cfg.input_kind = match args.input_kind() {
            cli::InputKind::Stdin => Input::Stdin,
        };

        cfg
    }
}
