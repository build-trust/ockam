use crate::util::{embedded_node, multiaddr_to_route};
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use ockam::{Context, TcpTransport};
use ockam_api::auth;
use ockam_api::auth::types::Attributes;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct AuthenticatedCommand {
    #[clap(subcommand)]
    subcommand: AuthenticatedSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthenticatedSubcommand {
    /// Set authenticated attributes.
    Set {
        /// Address to connect to.
        #[clap(long)]
        addr: MultiAddr,

        /// Subject identifier
        #[clap(long, validator(non_empty))]
        id: String,

        /// Attributes (use '=' to separate key from value).
        #[clap(value_delimiter('='))]
        attrs: Vec<String>,
    },
    /// Get attribute value.
    Get {
        /// Address to connect to.
        #[clap(long)]
        addr: MultiAddr,

        /// Subject identifier
        #[clap(long, validator(non_empty))]
        id: String,

        /// Attribute key.
        #[clap(validator(non_empty))]
        key: String,
    },
    /// Delete attribute
    Del {
        /// Address to connect to.
        #[clap(long)]
        addr: MultiAddr,

        /// Subject identifier
        #[clap(long, validator(non_empty))]
        id: String,

        /// Attribute key.
        #[clap(validator(non_empty))]
        key: String,
    },
}

impl AuthenticatedCommand {
    pub fn run(c: AuthenticatedCommand) {
        embedded_node(run_impl, c.subcommand)
    }
}

async fn run_impl(mut ctx: Context, cmd: AuthenticatedSubcommand) -> anyhow::Result<()> {
    TcpTransport::create(&ctx).await?;
    match &cmd {
        AuthenticatedSubcommand::Set { addr, id, attrs } => {
            let mut c = client(addr, &ctx).await?;
            let mut a = Attributes::new();
            for entry in attrs.chunks(2) {
                if let [k, v] = entry {
                    a.put(k, v.as_bytes());
                } else {
                    return Err(anyhow!("{entry:?} is not a key-value pair"));
                }
            }
            c.set(id, &a).await?
        }
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
    ctx.stop().await?;
    Ok(())
}

async fn client(addr: &MultiAddr, ctx: &Context) -> Result<auth::Client> {
    let to = multiaddr_to_route(addr).ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
    let cl = auth::Client::new(to, ctx).await?;
    Ok(cl)
}

fn non_empty(arg: &str) -> Result<(), String> {
    if arg.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(())
}
