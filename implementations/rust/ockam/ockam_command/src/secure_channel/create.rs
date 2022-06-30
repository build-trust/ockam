use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::{Args, Subcommand};
use ockam::Context;
use ockam_api::Status;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    #[clap(subcommand)]
    pub create_subcommand: CreateSubCommand,
}

impl CreateCommand {
    /// Get the peer/bind payload from this create command
    pub(crate) fn addr(&self) -> MultiAddr {
        match self.create_subcommand {
            CreateSubCommand::Connector { ref addr, .. } => addr.clone(),
            CreateSubCommand::Listener { ref bind, .. } => bind.clone(),
        }
    }
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
        bind: MultiAddr,
        /// Give this portal endpoint a name
        alias: Option<String>,
    },
}

impl CreateCommand {
    pub fn run(cfg: &OckamConfig, command: CreateCommand) -> anyhow::Result<()> {
        let port = cfg.select_node(&command.api_node).unwrap().port;

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
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            api::create_secure_channel(&cmd)?,
        )
        .await?;

    let (response, result) = api::parse_create_secure_channel_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!(
                "Secure Channel created! You can send messages to it via this address:\n{}",
                result.addr
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
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            api::create_secure_channel_listener(&cmd)?,
        )
        .await?;

    let response = api::parse_create_secure_channel_listener_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!(
                "Secure Channel Listener created! You can send messages to it via this address:\n{}",
                cmd.addr()
            )
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",)
        }
    }

    stop_node(ctx).await
}
