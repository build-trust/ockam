use crate::{args::OutletOpts, identity, storage};
use ockam::{identity::Identity, remote::RemoteForwarder, Context, TcpTransport, TCP};

pub async fn run(args: OutletOpts, ctx: Context) -> anyhow::Result<()> {
    crate::storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let (exported_ident, vault) = identity::load_identity_and_vault(&ockam_dir)?;
    let policy = storage::load_trust_policy(&ockam_dir)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let mut identity = Identity::import(&ctx, &vault, exported_ident).await?;
    identity
        .create_secure_channel_listener("secure_channel_listener", policy)
        .await?;

    tcp.create_outlet("outlet", &args.outlet_target).await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, &args.cloud_addr), &args.alias).await?;

    Ok(())
}
