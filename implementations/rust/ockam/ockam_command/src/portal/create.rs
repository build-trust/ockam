use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use ockam::{Context, Route};
use ockam_api::{
    nodes::types::{PortalStatus, PortalType},
    nodes::NODEMAN_ADDR,
    Status,
};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Select a creation variant
    #[clap(subcommand)]
    pub create_subcommand: CreateTypeCommand,

    /// Give this portal endpoint a name.  If none is provided a
    /// random one will be generated.
    pub alias: Option<String>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateTypeCommand {
    /// Create a TCP portal inlet
    TcpInlet {
        /// Portal inlet bind address
        bind: String,
        /// Forwarding point for the portal (ockam routing address)
        forward: MultiAddr,
    },
    /// Create a TCP portal outlet
    TcpOutlet {
        /// Portal outlet connection address
        address: String,
    },
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, command, create_portal)
    }
}

pub async fn create_portal(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::create_portal(&cmd)?,
        )
        .await?;

    let (
        response,
        PortalStatus {
            tt, addr, alias, ..
        },
    ) = api::parse_portal_status(&resp)?;

    match response.status() {
        Some(Status::Ok) if tt == PortalType::Inlet => {
            eprintln!(
                "Portal inlet '{}' created! You can send messages to it on this bind:\n{}`",
                alias, addr
            )
        }
        Some(Status::Ok) if tt == PortalType::Outlet => {
            let r: Route = base_route
                .modify()
                .pop_back()
                .append(addr.to_string())
                .into();
            eprintln!(
                "Portal outlet '{}' created! You can send messages through it via this route:\n{}",
                alias,
                ockam_api::route_to_multiaddr(&r).unwrap()
            );
        }

        _ => eprintln!("An unknown error occured while creating the portal component..."),
    }

    stop_node(ctx).await
}
