use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::util::{ComposableSnippet, Operation, Protocol, RemoteMode, exitcode};
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use ockam::{Context, Route, TCP};
use ockam_api::{
    nodes::{models::transport::TransportStatus, NODEMANAGER_ADDR},
    route_to_multiaddr, Status,
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, hidden = true)]
    reuse: bool,

    /// Select a creation variant
    #[clap(subcommand)]
    pub create_subcommand: CreateTypeCommand,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let mode = cc.create_subcommand.mode();
        // In the future we need to support other transport types
        let tcp = true;
        let address = cc.create_subcommand.address();

        Self {
            id: format!(
                "_transport_{}_{}_{}",
                mode,
                if tcp { "tcp" } else { "unknown" },
                address
            ),
            op: Operation::Transport {
                protocol: Protocol::Tcp,
                address,
                mode,
            },
            params: vec![],
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateTypeCommand {
    /// Create a TCP transport listener
    ///
    /// Listens for incoming TCP connections on the given bind address
    /// and port
    TcpListener {
        /// Transport connection or bind address
        bind: String,
    },
    /// Create a TCP transport connector
    ///
    /// Attempts to connect to an existing TCP transport listener on
    /// the given peer address and port
    TcpConnector {
        /// Transport connection or bind address
        address: String,
    },
}

impl CreateTypeCommand {
    fn mode(&self) -> RemoteMode {
        match self {
            Self::TcpListener { .. } => RemoteMode::Listener,
            Self::TcpConnector { .. } => RemoteMode::Connector,
        }
    }
    fn address(&self) -> String {
        match self {
            Self::TcpListener { bind } => bind.clone(),
            Self::TcpConnector { address } => address.clone(),
        }
    }
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        connect_to(port, command.clone(), create_transport);

        if !command.reuse {
            // Update the config.  We can probably assume that everything
            // went OK if we reach this point because embedded_node
            // crashes the process if something went wrong?  But idk,
            // still bad and should be fixed
            let composite = (&command).into();
            let node = command.node_opts.api_node;

            // Update the startup config
            let startup_cfg = match cfg.get_startup_cfg(&node) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("failed to load startup configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            };

            startup_cfg.add_composite(composite);
            if let Err(e) = startup_cfg.atomic_update().run() {
                eprintln!("failed to update configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }
        }
    }
}

pub async fn create_transport(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_transport(&cmd)?,
        )
        .await
        .expect("failed to send, or receive message");

    let (response, TransportStatus { payload, .. }) = api::parse_transport_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            let r: Route = base_route
                .modify()
                .pop_back()
                .append_t(TCP, payload.to_string())
                .into();

            eprintln!(
                "Transport created! You can send messages to it via this route:\n{}`",
                route_to_multiaddr(&r).unwrap(),
            )
        }
        _ => {
            eprintln!(
                "An error occurred while creating the transport: {}",
                payload
            );
            std::process::exit(exitcode::CANTCREAT);
        }
    }

    stop_node(ctx).await
}
