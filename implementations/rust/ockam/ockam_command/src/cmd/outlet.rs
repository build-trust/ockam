use crate::session::responder::SessionResponder;
use crate::{args::OutletOpts, identity, storage, OckamVault};
use ockam::{identity::Identity, remote::RemoteForwarder, Context, TcpTransport, TCP};
use ockam_vault::storage::FileStorage;
use std::sync::Arc;

pub async fn run(args: OutletOpts, ctx: Context) -> anyhow::Result<()> {
    crate::storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let vault_storage = FileStorage::create(&ockam_dir.join("vault.json")).await?;
    let vault = OckamVault::new(Some(Arc::new(vault_storage)));

    let exported_ident = identity::load_identity(&ockam_dir)?;
    let policy = storage::load_trust_policy(&ockam_dir)?;

    ctx.start_worker("session_responder", SessionResponder)
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;

    let identity = Identity::import(&ctx, &vault, exported_ident).await?;
    identity
        .create_secure_channel_listener("secure_channel_listener", policy)
        .await?;

    tcp.create_outlet("outlet", &args.outlet_target).await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, &args.cloud_addr), &args.alias).await?;

    Ok(())
}
