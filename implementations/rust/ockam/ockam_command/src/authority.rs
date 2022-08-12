use crate::{util::{connect_to, stop_node}, node::NodeOpts, CommandGlobalOpts};
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use data_encoding::BASE32_DNSSEC;
use ockam::{Context, Route};
use ockam_api::signer;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct AuthorityCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(subcommand)]
    subcommand: AuthoritySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthoritySubcommand {
    Get {
        #[clap(long, short)]
        addr: MultiAddr
    },
    Add {
        #[clap(long, short)]
        addr: MultiAddr,
        
        #[clap(long, short)]
        identity: String,
    }
}

impl AuthorityCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> Result<()> {
        let cfg = opts.config.get_node(&self.node_opts.api_node)?;
        connect_to(cfg.port, self, run_impl);
        Ok(())
    }
}

async fn run_impl(ctx: Context, cmd: AuthorityCommand, mut route: Route) -> Result<()> {
    let action = || async {
        match cmd.subcommand {
            AuthoritySubcommand::Get { addr } => {
                let pref = ockam_api::multiaddr_to_route(&addr)
                    .ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
                let route = route.modify().prepend_route(pref).append("signer").into();
                let mut client = signer::Client::new(route, &ctx).await?;
                let response = client.identity().await?;
                println!("{}", BASE32_DNSSEC.encode(response.identity()))
            }
            AuthoritySubcommand::Add { addr, identity } => {
                let bytes = BASE32_DNSSEC.decode(identity.as_bytes())?;
                let pref = ockam_api::multiaddr_to_route(&addr)
                    .ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
                let route = route.modify().prepend_route(pref).append("signer").into();
                let mut client = signer::Client::new(route, &ctx).await?;
                client.add_identity(&bytes).await?
            }
        }
        Ok(())
    };
    let result = action().await;
    stop_node(ctx).await?;
    result
}
