use crate::util::{api, connect_to, exitcode, stop_node};
use crate::CommandGlobalOpts;

use clap::Args;

use ockam::identity::IdentityIdentifier;

use super::with_at_opt;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;
use ockam_core::{Address, Route};
#[derive(Clone, Debug, Args)]

pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: with_at_opt::WithAtNodeOpt,

    /// Address for this listener
    address: Address,
    /// Authorized Identifiers of secure channel initators
    #[clap(short, long, value_name = "IDENTIFIER")]
    authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.at) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        connect_to(port, command, create_listener);

        Ok(())
    }
}

pub async fn create_listener(
    ctx: ockam::Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let CreateCommand {
        address: addr,
        authorized_identifier: authorized_identifiers,
        ..
    } = cmd;

    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel_listener(&addr, authorized_identifiers)?,
        )
        .await?;

    let response = api::parse_create_secure_channel_listener_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("/service/{}", addr.address())
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",);
            std::process::exit(exitcode::CANTCREAT)
        }
    }

    stop_node(ctx).await
}
