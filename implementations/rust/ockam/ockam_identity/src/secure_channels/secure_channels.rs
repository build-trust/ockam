use crate::identities::Identities;
use crate::identities::IdentitiesVault;
use crate::identity::IdentityIdentifier;
use crate::secure_channel::handshake_worker::HandshakeWorker;
use crate::secure_channel::{
    Addresses, IdentityChannelListener, Role, SecureChannelListenerOptions, SecureChannelOptions,
    SecureChannelRegistry,
};
use crate::{SecureChannel, SecureChannelListener, SecureChannelsBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::sync::{AtomicBool, Ordering};
use ockam_core::AsyncTryClone;
use ockam_core::Result;
use ockam_core::{Address, Route};
use ockam_node::compat::tokio::time::sleep;
use ockam_node::{spawn, Context};

/// Identity implementation
#[derive(Clone)]
pub struct SecureChannels {
    pub(crate) identities: Arc<Identities>,
    pub(crate) secure_channel_registry: SecureChannelRegistry,
}

impl SecureChannels {
    /// Constructor
    pub(crate) fn new(
        identities: Arc<Identities>,
        secure_channel_registry: SecureChannelRegistry,
    ) -> Self {
        Self {
            identities,
            secure_channel_registry,
        }
    }

    /// Return the identities services associated to this service
    pub fn identities(&self) -> Arc<Identities> {
        self.identities.clone()
    }

    /// Return the vault associated to this service
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.identities.vault.clone()
    }

    /// Return the secure channel registry
    pub fn secure_channel_registry(&self) -> SecureChannelRegistry {
        self.secure_channel_registry.clone()
    }

    /// Create a builder for secure channels
    pub fn builder() -> SecureChannelsBuilder {
        SecureChannelsBuilder {
            identities_builder: Identities::builder(),
            registry: SecureChannelRegistry::new(),
        }
    }
}

impl SecureChannels {
    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub async fn create_secure_channel_listener(
        &self,
        ctx: &Context,
        identifier: &IdentityIdentifier,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<SecureChannelListener> {
        let address = address.into();
        let options = options.into();
        let flow_control_id = options.flow_control_id.clone();

        IdentityChannelListener::create(
            ctx,
            Arc::new(self.clone()),
            identifier,
            address.clone(),
            options,
        )
        .await?;

        Ok(SecureChannelListener::new(address, flow_control_id))
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
        identifier: &IdentityIdentifier,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<SecureChannel> {
        let addresses = Addresses::generate(Role::Initiator);
        let options = options.into();
        let flow_control_id = options.flow_control_id.clone();
        let route = route.into();
        let next = route.next()?;
        let is_idle = Arc::new(AtomicBool::new(true));
        let maximum_idle_time = options.maximum_idle_time;
        options.setup_flow_control(ctx.flow_controls(), &addresses, next)?;
        let access_control = options.create_access_control(ctx.flow_controls());

        HandshakeWorker::create(
            ctx,
            Arc::new(self.clone()),
            addresses.clone(),
            identifier.clone(),
            options.trust_policy,
            access_control.decryptor_outgoing_access_control,
            options.credentials,
            options.trust_context,
            Some(route),
            Some(options.timeout),
            Some(Arc::clone(&is_idle)),
            Role::Initiator,
        )
        .await?;

        let self_clone = self.clone();
        let ctx_clone = ctx.async_try_clone().await?;
        let addr = addresses.encryptor.clone();

        // start a background thread to observe the connection activity
        // If a message is received before maximum_idle_time has passed then the connection is
        // active. Otherwise, we close the secure channel
        spawn(async move {
            loop {
                sleep(maximum_idle_time).await;
                if is_idle.load(Ordering::Relaxed) {
                    self_clone
                        .stop_secure_channel(&ctx_clone, &addr)
                        .await
                        .unwrap();
                    break;
                }
                is_idle.store(true, Ordering::Relaxed);
            }
        });

        Ok(SecureChannel::new(
            addresses.encryptor,
            addresses.encryptor_api,
            flow_control_id,
        ))
    }

    /// Stop a SecureChannel given an encryptor address
    pub async fn stop_secure_channel(&self, ctx: &Context, channel: &Address) -> Result<()> {
        ctx.stop_worker(channel.clone()).await
    }
}
