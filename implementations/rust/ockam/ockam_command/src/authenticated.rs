use crate::help;
use crate::util::embedded_node;
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use ockam::{Context, TcpTransport};
use ockam_api::auth;
use ockam_multiaddr::MultiAddr;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide(), help_template = help::template(HELP_DETAIL))]
pub struct AuthenticatedCommand {
    #[clap(subcommand)]
    subcommand: AuthenticatedSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthenticatedSubcommand {
    /// Get attribute value.
    Get {
        /// Address to connect to.
        addr: MultiAddr,

        /// Subject identifier
        #[clap(long, forbid_empty_values = true)]
        id: String,

        /// Attribute key.
        #[clap(forbid_empty_values = true)]
        key: String,
    },
    /// Delete attribute
    Del {
        /// Address to connect to.
        addr: MultiAddr,

        /// Subject identifier
        #[clap(long, forbid_empty_values = true)]
        id: String,

        /// Attribute key.
        #[clap(forbid_empty_values = true)]
        key: String,
    },
}

impl AuthenticatedCommand {
    pub fn run(self) {
        if let Err(e) = embedded_node(run_impl, self.subcommand) {
            eprintln!("Ockam node failed: {:?}", e,);
        }
    }
}

async fn run_impl(ctx: Context, cmd: AuthenticatedSubcommand) -> crate::Result<()> {
    TcpTransport::create(&ctx).await?;
    match &cmd {
        AuthenticatedSubcommand::Get { addr, id, key } => {
            let mut c = client(addr, &ctx).await?;
            let val = c.get(id, key).await?;
            println!("{val:?}")
        }
        AuthenticatedSubcommand::Del { addr, id, key } => {
            let mut c = client(addr, &ctx).await?;
            c.del(id, key).await?;
        }
    }

    Ok(())
}

async fn client(addr: &MultiAddr, ctx: &Context) -> Result<auth::Client> {
    let to = ockam_api::multiaddr_to_route(addr)
        .ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
    let cl = auth::Client::new(to, ctx).await?;
    Ok(cl)
}
