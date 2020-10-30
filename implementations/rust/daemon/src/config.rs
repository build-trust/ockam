use std::net::SocketAddr;
use std::path::PathBuf;

use crate::cli;

use crate::cli::VaultKind;
use ockam_kex::CipherSuite;
use ockam_message::message::Route;

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Source,
    Sink,
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
    vault_kind: VaultKind,
    onward_route: Option<Route>,
    route_hub: Option<SocketAddr>, // TODO: make this a Route so it can be multiple hops.
    output_to_stdout: bool,
    local_socket: SocketAddr,
    // router_socket: Option<SocketAddr>,
    // channel_to_sink: Option<String>,
    role: Role,
    vault_path: PathBuf,
    input_kind: Input,
    public_key_sink: Option<String>,
    public_key_hub: Option<String>,
    service_address: Option<String>,
    identity_name: String,
    addon: Option<AddonKind>,
    cipher_suite: CipherSuite,
}

impl Default for Config {
    fn default() -> Self {
        cli::Args::default().into()
    }
}

impl Config {
    pub fn vault_kind(&self) -> VaultKind {
        self.vault_kind
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }

    pub fn onward_route(&self) -> Option<Route> {
        self.onward_route.clone()
    }

    pub fn route_hub(&self) -> Option<SocketAddr> {
        self.route_hub.clone()
    }

    pub fn input_kind(&self) -> Input {
        self.input_kind
    }

    pub fn local_socket(&self) -> SocketAddr {
        self.local_socket
    }

    // pub fn router_socket(&self) -> Option<SocketAddr> {
    //     self.router_socket
    // }

    // pub fn channel_to_sink(&self) -> Option<String> {
    //     self.channel_to_sink.clone()
    // }

    pub fn public_key_sink(&self) -> Option<String> {
        self.public_key_sink.clone()
    }

    pub fn public_key_hub(&self) -> Option<String> {
        self.public_key_hub.clone()
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

    pub fn cipher_suite(&self) -> CipherSuite {
        self.cipher_suite
    }
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            vault_kind: args.vault_kind(),
            onward_route: None,
            route_hub: args.route_hub(),
            output_to_stdout: false,
            local_socket: args.local_socket(),
            // channel_to_sink: args.channel_to_sink(),
            // router_socket: args.router_socket(),
            role: Role::Source,
            vault_path: args.vault_path(),
            input_kind: Input::Stdin,
            public_key_sink: args.public_key_sink(),
            public_key_hub: args.public_key_hub(),
            service_address: args.service_address(),
            identity_name: args.identity_name(),
            addon: if let Some(a) = args.addon() {
                match a {
                    cli::Addon::InfluxDb(u, db) => Some(AddonKind::InfluxDb(u, db)),
                }
            } else {
                None
            },
            cipher_suite: args.cipher_suite(),
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
            cli::ChannelRole::Source => Role::Source,
            cli::ChannelRole::Sink => Role::Sink,
            cli::ChannelRole::Router => Role::Router,
        };

        cfg.input_kind = match args.input_kind() {
            cli::InputKind::Stdin => Input::Stdin,
        };

        cfg
    }
}
