use clap::Args;

use ockam::access_control::{AnyAccessControl, LocalOriginOnly};
use ockam::{remote::RemoteForwarder, Context, TcpTransport, TCP};

use crate::old::session::responder::SessionResponder;
use crate::old::{identity, storage, OckamStorage};

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

    let (policy, access_control) = storage::load_trust_policy(&ockam_dir)?;

    let access_control = AnyAccessControl::new(access_control, LocalOriginOnly);

    // TODO: AccessControl

    ctx.start_worker("session_responder", SessionResponder)
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;

    let identity = identity::load_identity(&ctx, &ockam_dir).await?;

    identity
        .create_secure_channel_listener("secure_channel_listener", policy, &OckamStorage::new())
        .await?;

    tcp.create_outlet_with_access_control("outlet", &args.outlet_target, access_control)
        .await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, &args.cloud_addr), &args.alias).await?;

    Ok(())
}
