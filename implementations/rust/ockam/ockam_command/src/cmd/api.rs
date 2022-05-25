use crate::args::{Api, Nodes};
use crate::identity;
use crate::storage;
use crate::util::multiaddr_to_route;
use crate::OckamVault;
use anyhow::anyhow;
use ockam::identity::{Identity, TrustMultiIdentifiersPolicy};
use ockam::vault::storage::FileStorage;
use ockam::vault::Vault;
use ockam::{route, Context, TcpTransport};
use ockam_api::nodes;
use ockam_api::nodes::types::CreateNode;
use ockam_multiaddr::MultiAddr;
use std::sync::Arc;

pub async fn run(cmd: Api, mut ctx: Context) -> anyhow::Result<()> {
    storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let vault = {
        let storage = FileStorage::create(
            &ockam_dir.join("vault.json"),
            &ockam_dir.join("vault.json.temp"),
        )
        .await?;
        OckamVault::new(Some(Arc::new(storage)))
    };

    let id = identity::load_identity(&ockam_dir)?;
    let policy = storage::load_trust_policy(&ockam_dir)?;
    let identity = Identity::import(&ctx, &vault, id).await?;
    TcpTransport::create(&ctx).await?;

    match cmd {
        Api::Nodes(cmd) => match cmd {
            Nodes::Create { addr, name } => {
                let mut c = client(&addr, &identity, &ctx, policy).await?;
                let info = c.create_node(&CreateNode::new(name)).await?;
                println!("{info:?}")
            }
            Nodes::Get { addr, id } => {
                let mut c = client(&addr, &identity, &ctx, policy).await?;
                let info = c.get(&id).await?;
                println!("{info:?}")
            }
            Nodes::List { addr } => {
                let mut c = client(&addr, &identity, &ctx, policy).await?;
                let info = c.list().await?;
                println!("{info:?}")
            }
            Nodes::Delete { addr, id } => {
                let mut c = client(&addr, &identity, &ctx, policy).await?;
                let info = c.delete(&id).await?;
                println!("{info:?}")
            }
        },
    }

    ctx.stop().await?;
    Ok(())
}

async fn client(
    addr: &MultiAddr,
    id: &Identity<Vault>,
    ctx: &Context,
    policy: TrustMultiIdentifiersPolicy,
) -> anyhow::Result<nodes::Client> {
    let to = multiaddr_to_route(addr).ok_or_else(|| anyhow!("failed to parse address: {addr}"))?;
    let me = id.create_secure_channel(to, policy).await?;
    let to = route![me, "nodes"];
    let cl = nodes::Client::new(to, ctx).await?;
    Ok(cl)
}
