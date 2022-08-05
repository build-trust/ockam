use clap::Args;
use std::io;
use std::str::FromStr;

use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    addr: String,
    message: String,
}

impl SendCommand {
    pub fn run(cmd: SendCommand) {
        embedded_node(send_message, cmd)
    }
}

async fn send_message(mut ctx: Context, cmd: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let addr = match cmd.addr {
        addr if addr.contains("/-") => {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            let args_from_stdin = buffer
                .trim()
                .split('/')
                .filter(|&s| !s.is_empty())
                .fold("".to_owned(), |acc, s| format!("{acc}/{s}"));

            addr.replace("/-", &args_from_stdin)
        }
        addr if addr.contains("-/") => {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;

            let args_from_stdin = buffer
                .trim()
                .split('/')
                .filter(|&s| !s.is_empty())
                .fold("/".to_owned(), |acc, s| format!("{acc}{s}/"));

            addr.replace("-/", &args_from_stdin)
        }
        _ => cmd.addr,
    };

    let addr = MultiAddr::from_str(&addr)?;
    if let Some(route) = ockam_api::multiaddr_to_route(&addr) {
        ctx.send(route, cmd.message).await?;
        let message = ctx.receive::<String>().await?;
        println!("{}", message);
    }

    ctx.stop().await?;

    Ok(())
}
