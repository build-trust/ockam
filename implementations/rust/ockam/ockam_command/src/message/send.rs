use clap::Args;
use std::io;
use std::str::FromStr;

use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

use crate::util::embedded_node;
#[derive(Clone, Debug, Args)]
pub struct SendCommand {
    #[clap(flatten)]
    addr: HyphenatedMultiAddr,
    message: String,
}

impl SendCommand {
    pub fn run(cmd: SendCommand) {
        embedded_node(send_message, cmd)
    }
}

#[derive(Clone, Debug, Args)]
struct HyphenatedMultiAddr {
    addr: String,
}

impl HyphenatedMultiAddr {
    #[allow(dead_code)]
    fn new(addr: &str) -> Self {
        HyphenatedMultiAddr {
            addr: addr.to_string(),
        }
    }

    fn multiaddr(&self) -> Result<MultiAddr, anyhow::Error> {
        let addr = match &self.addr {
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
            _ => self.addr.to_owned(),
        };

        let multi_addr = MultiAddr::from_str(&addr)?;
        Ok(multi_addr)
    }
}

async fn send_message(mut ctx: Context, cmd: SendCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let multi_addr = cmd.addr.multiaddr()?;
    if let Some(route) = ockam_api::multiaddr_to_route(&multi_addr) {
        ctx.send(route, cmd.message).await?;
        let message = ctx.receive::<String>().await?;
        println!("{}", message);
    }

    ctx.stop().await?;

    Ok(())
}
