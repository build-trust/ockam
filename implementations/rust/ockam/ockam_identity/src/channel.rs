mod addresses;
mod decryptor;
mod decryptor_state;
mod decryptor_worker;
mod encryptor;
mod encryptor_worker;
mod listener;
mod messages;

mod common;
mod local_info;
mod registry;
mod trust_options;
mod trust_policy;

pub use common::*;
pub use local_info::*;
pub use registry::*;
pub use trust_options::*;
pub use trust_policy::*;

/// `AccessControl` implementation based on SecureChannel authentication guarantees
pub mod access_control;
/// SecureChannel API
pub mod api;

use crate::authenticated_storage::AuthenticatedStorage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::channel::listener::IdentityChannelListener;
use crate::error::IdentityError;
use crate::{Identity, IdentityVault};
use core::time::Duration;
use ockam_core::{Address, AsyncTryClone, Result, Route};

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// Spawns a SecureChannel listener at given `Address`
    pub async fn create_secure_channel_listener(
        &self,
        address: impl Into<Address>,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        self.create_secure_channel_listener_trust(
            address,
            SecureChannelListenerTrustOptions::new().with_trust_policy(trust_policy),
        )
        .await
    }

    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerTrustOptions`]
    pub async fn create_secure_channel_listener_trust(
        &self,
        address: impl Into<Address>,
        trust_options: SecureChannelListenerTrustOptions,
    ) -> Result<()> {
        let identity_clone = self.async_try_clone().await?;

        IdentityChannelListener::create(&self.ctx, address.into(), trust_options, identity_clone)
            .await?;

        Ok(())
    }

    /// Initiate a SecureChannel using [`Route`] to the SecureChannel listener
    pub async fn create_secure_channel(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address> {
        self.create_secure_channel_trust(
            route,
            SecureChannelTrustOptions::new().with_trust_policy(trust_policy),
        )
        .await
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelTrustOptions`]
    pub async fn create_secure_channel_trust(
        &self,
        route: impl Into<Route>,
        trust_options: SecureChannelTrustOptions,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            trust_options,
            Duration::from_secs(120),
        )
        .await
    }

    /// Extended function to create a SecureChannel
    pub async fn create_secure_channel_extended(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
        timeout: Duration,
    ) -> Result<Address> {
        self.create_secure_channel_extended_trust(
            route,
            SecureChannelTrustOptions::new().with_trust_policy(trust_policy),
            timeout,
        )
        .await
    }

    /// Extended function to create a SecureChannel with [`SecureChannelTrustOptions`]
    pub async fn create_secure_channel_extended_trust(
        &self,
        route: impl Into<Route>,
        trust_options: SecureChannelTrustOptions,
        timeout: Duration,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            trust_options,
            timeout,
        )
        .await
    }

    /// Stop a SecureChannel given an encryptor address
    pub async fn stop_secure_channel(&self, channel: &Address) -> Result<()> {
        if let Some(entry) = self.secure_channel_registry.unregister_channel(channel) {
            let err1 = self
                .ctx
                .stop_worker(entry.encryptor_messaging_address().clone())
                .await
                .err();
            let err2 = self
                .ctx
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
