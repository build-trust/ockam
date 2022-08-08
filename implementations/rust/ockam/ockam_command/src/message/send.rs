use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use crate::{embedded_node, CommandGlobalOpts};
use ockam::TcpTransport;
use ockam_api::clean_multiaddr;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Response, Status};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

use crate::util::{api, connect_to, exitcode, stop_node};

#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    /// The node to send messages from
    #[clap(short, long, value_name = "NODE")]
    from: Option<String>,

    /// The route to send the message to
    #[clap(short, long, value_name = "ROUTE")]
    pub to: MultiAddr,

    pub message: String,
}

impl SendCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: SendCommand) {
        // First we clean the MultiAddr route to replace /node/<foo>
        // with the address lookup for `<foo>`
        let cmd = SendCommand {
            to: match clean_multiaddr(&cmd.to, &opts.config.get_lookup()) {
                Some(to) => to,
                None => {
                    eprintln!("failed to normalize MultiAddr route");
                    std::process::exit(exitcode::USAGE);
                }
            },
            ..cmd
        };

        if let Some(node) = &cmd.from {
            let port = opts.config.get_node_port(node);
            connect_to(port, (opts, cmd), send_message_via_connection_to_a_node);
        } else if let Err(e) = embedded_node(send_message_from_embedded_node, cmd) {
            eprintln!("Ockam node failed: {:?}", e,);
        }
    }
}

async fn send_message_from_embedded_node(
    mut ctx: ockam::Context,
    cmd: SendCommand,
) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    if let Some(route) = ockam_api::multiaddr_to_route(&cmd.to) {
        ctx.send(route, cmd.message).await?;
        let message = ctx.receive::<String>().await?;
        println!("{}", message);
    }

    ctx.stop().await?;

    Ok(())
}

async fn send_message_via_connection_to_a_node(
    ctx: ockam::Context,
    (_opts, cmd): (CommandGlobalOpts, SendCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::message::send(cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec
        .decode::<Response>()
        .context("Failed to decode Response")?;
    debug!(?header, "Received response");

    let res = match (header.status(), header.has_body()) {
        (Some(Status::Ok), true) => {
            let body = dec
                .decode::<Vec<u8>>()
                .context("Failed to decode response body")?;
            Ok(String::from_utf8(body)?)
        }
        (Some(status), true) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request with status code {status:?}: {err}"
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
