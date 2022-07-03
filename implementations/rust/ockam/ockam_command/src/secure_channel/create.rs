use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use ockam::Context;
use ockam_api::error::ApiError;
use ockam_api::{route_to_multiaddr, Status};
use ockam_core::{route, Route};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    #[clap(subcommand)]
    pub create_subcommand: CreateSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateSubCommand {
    /// Connect to an existing secure channel listener
    Connector {
        /// What address to connect to
        addr: MultiAddr,
        /// Give this portal endpoint a name
        alias: Option<String>,
    },
    /// Create a new secure channel listener
    Listener {
        /// Specify an address for this listener
        bind: String,
        /// Give this portal endpoint a name
        alias: Option<String>,
    },
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        match command.create_subcommand {
            CreateSubCommand::Connector { .. } => connect_to(port, command, create_connector),
            CreateSubCommand::Listener { .. } => connect_to(port, command, create_listener),
        }

        Ok(())
    }
}

pub async fn create_connector(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        CreateSubCommand::Connector { addr, .. } => addr,
        CreateSubCommand::Listener { .. } => {
            return Err(ApiError::generic("Internal logic error").into())
        }
    };

    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            api::create_secure_channel(&addr)?,
        )
        .await?;

    let (response, result) = api::parse_create_secure_channel_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            let addr = route_to_multiaddr(&route![result.addr.to_string()])
                .ok_or_else(|| ApiError::generic("Invalid Secure Channel Address"))?;
            eprintln!(
                "Secure Channel created! You can send messages to it via this address:\n{}",
                addr
            )
        }
        _ => {
            eprintln!("An error occurred while creating secure channel",)
        }
    }

    stop_node(ctx).await
}

pub async fn create_listener(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        CreateSubCommand::Connector { .. } => {
            return Err(ApiError::generic("Internal logic error").into())
        }
        CreateSubCommand::Listener { bind, .. } => bind,
    };

    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            api::create_secure_channel_listener(&addr)?,
        )
        .await?;

    let response = api::parse_create_secure_channel_listener_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!("Secure Channel Listener created at {}!", addr)
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",)
        }
    }

    stop_node(ctx).await
}
