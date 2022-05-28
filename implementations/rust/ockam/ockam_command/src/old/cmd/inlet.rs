use crate::old::session::initiator::{SessionMaintainer, SessionManager};
use crate::old::{identity, storage, OckamVault};
use clap::Args;
use ockam::{identity::*, route, Context, Result, TcpTransport, TCP};
use ockam_core::{Address, AsyncTryClone, Route};
use ockam_vault::storage::FileStorage;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, Args)]
pub struct InletOpts {
    /// Ockam's cloud node address
    pub cloud_addr: String,
    /// Alias that is used to identify Control Plane node
    pub alias: String,
    /// Bind address for the inlet to listen on.
    pub inlet_address: String,
}

#[derive(Debug)]
struct ExistingSession {
    pub _channel: Address,
    pub inlet_address: Address,
}

struct InletSessionManager {
    args: InletOpts,
    tcp: TcpTransport,
    identity: Identity<OckamVault>,
    policy: TrustMultiIdentifiersPolicy,
    existing_session: Option<ExistingSession>,
}

impl InletSessionManager {
    pub fn new(
        args: InletOpts,
        tcp: TcpTransport,
        identity: Identity<OckamVault>,
        policy: TrustMultiIdentifiersPolicy,
    ) -> Self {
        Self {
            args,
            tcp,
            identity,
            policy,
            existing_session: None,
        }
    }
}

#[ockam::worker]
impl SessionManager for InletSessionManager {
    async fn start_session(&mut self, _ctx: &Context, timeout: Duration) -> Result<Route> {
        let channel = self
            .identity
            .create_secure_channel_extended(
                route![
                    (TCP, &self.args.cloud_addr),
                    format!("forward_to_{}", self.args.alias),
                    "secure_channel_listener"
                ],
                self.policy.async_try_clone().await?,
                timeout,
            )
            .await?;

        let (inlet_address, _) = self
            .tcp
            .create_inlet(&self.args.inlet_address, route![channel.clone(), "outlet"])
            .await?;

        self.existing_session = Some(ExistingSession {
            _channel: channel.clone(),
            inlet_address,
        });

        let responder_route = route![channel, "session_responder"];

        Ok(responder_route)
    }

    async fn stop_session(&mut self, _ctx: &Context) -> Result<()> {
        if let Some(existing_session) = self.existing_session.take() {
            tracing::info!("Stopping session {:?}", existing_session);
            // TODO: Stop SecureChannel
            self.tcp.stop_inlet(existing_session.inlet_address).await?;
        }

        Ok(())
    }
}

pub async fn run(args: InletOpts, ctx: Context) -> anyhow::Result<()> {
    storage::ensure_identity_exists(true)?;
    let ockam_dir = storage::get_ockam_dir()?;

    let vault_storage = FileStorage::create(
        &ockam_dir.join("vault.json"),
        &ockam_dir.join("vault.json.temp"),
    )
    .await?;
    let vault = OckamVault::new(Some(Arc::new(vault_storage)));

    let exported_id = identity::load_identity(&ockam_dir)?;
    let policy = storage::load_trust_policy(&ockam_dir)?;

    let tcp = TcpTransport::create(&ctx).await?;
    let identity = Identity::import(&ctx, &vault, exported_id).await?;

    let session_manager = InletSessionManager::new(args, tcp, identity, policy);

    SessionMaintainer::start(&ctx, session_manager).await?;

    Ok(())
}
