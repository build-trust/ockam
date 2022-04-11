use crate::session::responder::SessionResponder;
use crate::{args::OutletOpts, identity, storage};
use ockam::access_control::{AnyAccessControl, LocalOriginOnly};
use ockam::{identity::Identity, remote::RemoteForwarder, Context, TcpTransport, TCP};
use std::sync::Arc;

pub async fn run(args: OutletOpts, ctx: Context) -> anyhow::Result<()> {
    crate::storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let (exported_ident, vault) = identity::load_identity_and_vault(&ockam_dir)?;
    let (policy, access_control) = storage::load_trust_policy(&ockam_dir)?;

    let access_control = AnyAccessControl::new(access_control, LocalOriginOnly);
    let access_control = Arc::new(access_control);

    // TODO: AccessControl

    ctx.start_worker("session_responder", SessionResponder)
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;

    let identity = Identity::import(&ctx, &vault, exported_ident).await?;
    identity
        .create_secure_channel_listener("secure_channel_listener", policy)
        .await?;

    tcp.create_outlet_with_access_control("outlet", &args.outlet_target, access_control)
        .await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, &args.cloud_addr), &args.alias).await?;

    Ok(())
}
