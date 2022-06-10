use crate::old::session::responder::SessionResponder;
use crate::old::{identity, storage, OckamVault};
use clap::Args;
use ockam::access_control::{AnyAccessControl, LocalOriginOnly};
use ockam::{identity::Identity, remote::RemoteForwarder, Context, TcpTransport, TCP};
use ockam_vault::storage::FileStorage;
use std::sync::Arc;

#[derive(Clone, Debug, Args)]
pub struct OutletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Address of tcp service running on Control Plane node that will receive
    /// connections from the Outlet
    pub outlet_target: String,
}

pub async fn run(args: OutletOpts, ctx: Context) -> anyhow::Result<()> {
    crate::old::storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let vault_storage = FileStorage::create(
        &ockam_dir.join("vault.json"),
        &ockam_dir.join("vault.json.temp"),
    )
    .await?;
    let vault = OckamVault::new(Some(Arc::new(vault_storage)));

    let exported_ident = identity::load_identity(&ockam_dir)?;
    let (policy, access_control) = storage::load_trust_policy(&ockam_dir)?;

    let access_control = AnyAccessControl::new(access_control, LocalOriginOnly);

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
