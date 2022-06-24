use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::Args;
use ockam::Context;
use ockam_api::Status;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateListenerCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Address of the Secure Channel Listener
    #[clap(long)]
    pub addr: MultiAddr,
}

impl CreateListenerCommand {
    pub fn run(cfg: &OckamConfig, command: CreateListenerCommand) -> anyhow::Result<()> {
        let port = cfg.select_node(&command.api_node).unwrap().port;

        connect_to(port, command, create_listener);

        Ok(())
    }
}

pub async fn create_listener(
    ctx: Context,
    cmd: CreateListenerCommand,
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
                cmd.addr
            )
        }
        _ => {
            eprintln!("An error occurred while creating secure channel listener",)
        }
    }

    stop_node(ctx).await
}
