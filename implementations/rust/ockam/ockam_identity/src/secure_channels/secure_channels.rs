use core::time::Duration;

use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_core::{Address, Route};
use ockam_node::Context;

use crate::identities::Identities;
use crate::identities::IdentitiesVault;
use crate::identity::IdentityError;
use crate::secure_channel::{
    Addresses, DecryptorWorker, IdentityChannelListener, Role, SecureChannelListenerOptions,
    SecureChannelOptions, SecureChannelRegistry,
};
use crate::{IdentityIdentifier, SecureChannelsBuilder};

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
    ) -> Result<()> {
        IdentityChannelListener::create(
            ctx,
            Arc::new(self.clone()),
            identifier,
            address.into(),
            options.into(),
        )
        .await?;

        Ok(())
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
        identifier: &IdentityIdentifier,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<Address> {
        let addresses = Addresses::generate(Role::Initiator);
        let options = options.into();

        let route = route.into();
        let next = route.next()?;
        options.setup_flow_control(ctx.flow_controls(), &addresses, next)?;
        let access_control = options.create_access_control(ctx.flow_controls());

        DecryptorWorker::create_initiator(
            ctx,
            Arc::new(self.clone()),
            identifier.clone(),
            route,
            addresses,
            options.trust_policy,
            access_control.decryptor_outgoing_access_control,
            Duration::from_secs(120),
        )
        .await
    }

    /// Extended function to create a SecureChannel with [`SecureChannelOptions`]
    pub async fn create_secure_channel_extended(
        &self,
        ctx: &Context,
        identifier: &IdentityIdentifier,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
        timeout: Duration,
    ) -> Result<Address> {
        let addresses = Addresses::generate(Role::Initiator);

        let route = route.into();
        let next = route.next()?;
        let options = options.into();
        options.setup_flow_control(ctx.flow_controls(), &addresses, next)?;
        let access_control = options.create_access_control(ctx.flow_controls());

        DecryptorWorker::create_initiator(
            ctx,
            Arc::new(self.clone()),
            identifier.clone(),
            route,
            addresses,
            options.trust_policy,
            access_control.decryptor_outgoing_access_control,
            timeout,
        )
        .await
    }

    /// Stop a SecureChannel given an encryptor address
    pub async fn stop_secure_channel(&self, ctx: &Context, channel: &Address) -> Result<()> {
        if let Some(entry) = self.secure_channel_registry.unregister_channel(channel) {
            let err1 = ctx
                .stop_worker(entry.encryptor_messaging_address().clone())
                .await
                .err();
            let err2 = ctx
                .stop_worker(entry.decryptor_messaging_address().clone())
                .await
                .err();

            if let Some(err1) = err1 {
                return Err(err1);
            }
            if let Some(err2) = err2 {
                return Err(err2);
            }
        } else {
            return Err(IdentityError::SecureChannelNotFound.into());
        }

        Ok(())
    }
}
