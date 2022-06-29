use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::Args;
use ockam::Context;
use ockam_api::Status;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Address of the Secure Channel Listener
    #[clap(long)]
    pub addr: MultiAddr,
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) -> anyhow::Result<()> {
        let port = cfg.select_node(&command.api_node).unwrap().port;

        connect_to(port, command, create_channel);

        Ok(())
    }
}

pub async fn create_channel(
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
