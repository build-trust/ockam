use crate::args::StartNodeOpts;
use crate::identity;
use crate::storage;
use crate::OckamVault;
use ockam::identity::Identity;
use ockam::vault::storage::FileStorage;
use ockam::{Context, TcpTransport};
use std::sync::Arc;

pub async fn run(args: StartNodeOpts, ctx: Context) -> anyhow::Result<()> {
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

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(&args.listen_addr).await?;

    identity
        .create_secure_channel_listener("api", policy)
        .await?;

    ctx.start_worker("nodes", ockam_api::nodes::Server::default())
        .await?;

    Ok(())
}
