use crate::storage::*;
use ockam::{
    AsyncTryClone, Context, Identity, RemoteForwarder, TcpTransport, TrustEveryonePolicy, TrustMultiIdentifiersPolicy,
    TCP,
};

pub async fn run(args: crate::args::OutletOpts, ctx: Context) -> anyhow::Result<()> {
    crate::storage::ensure_identity_exists(true)?;
    let ockam_dir = crate::storage::get_ockam_dir()?;

    let (exported_ident, vault) = crate::identity::load_identity_and_vault(&ockam_dir)?;
    let policy = crate::storage::load_trust_policy(&ockam_dir)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let mut identity = Identity::import(&ctx, &vault, exported_ident).await?;
    identity
        .create_secure_channel_listener("secure_channel_listener", policy)
        .await?;

    tcp.create_outlet("outlet", &args.outlet_target).await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, &args.cloud_addr), &args.alias).await?;

    Ok(())
}
