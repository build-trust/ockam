use crate::util::get_node_name_from_path;
use crate::util::{api, connect_to, exitcode, stop_node};
use crate::CommandGlobalOpts;
use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use ockam::identity::IdentityIdentifier;
use ockam_api::error::ApiError;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::{clean_multiaddr, route_to_multiaddr};
use ockam_core::api::{Response, Status};
use ockam_core::{route, Route};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: SecureChannelNodeOpts,

    /// Route to a secure channel listener (required)
    #[clap(name = "to", short, long, value_name = "ROUTE")]
    addr: MultiAddr,

    /// Pre-known Identifiers of the other side
    #[clap(short, long)]
    authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

#[derive(Clone, Debug, Args)]
pub struct SecureChannelNodeOpts {
    /// Node that will initiate the secure channel
    #[clap(
        global = true,
        short,
        long,
        value_name = "NODE",
        default_value = "default"
    )]
    pub from: String,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let input_value = get_node_name_from_path(&command.node_opts.from);
        let port = match cfg.select_node(input_value) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        let command = CreateCommand {
            addr: match clean_multiaddr(&command.addr, &cfg.get_lookup()) {
                Some(addr) => addr,
                None => {
                    eprintln!("failed to normalize MultiAddr route");
                    std::process::exit(exitcode::USAGE);
                }
            },
            node_opts: SecureChannelNodeOpts {
                from: input_value.to_string(),
            },
            ..command
        };

        connect_to(port, command, create_connector);

        Ok(())
    }
}

pub async fn create_connector(
    ctx: ockam::Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let CreateCommand {
        addr,
        authorized_identifier: authorized_identifiers,
        ..
    } = cmd;

    let response: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel(addr, authorized_identifiers)?,
        )
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<CreateSecureChannelResponse>()?;
            let addr = route_to_multiaddr(&route![body.addr.to_string()])
                .ok_or_else(|| ApiError::generic("Invalid Secure Channel Address"))?;
            Ok(addr)
        }
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}
