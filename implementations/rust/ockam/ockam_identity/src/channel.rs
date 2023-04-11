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
mod options;
mod registry;
mod trust_policy;

pub use common::*;
pub use local_info::*;
pub use options::*;
pub use registry::*;
pub use trust_policy::*;

/// `AccessControl` implementation based on SecureChannel authentication guarantees
pub mod access_control;
/// SecureChannel API
pub mod api;

use crate::channel::addresses::Addresses;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::channel::listener::IdentityChannelListener;
use crate::error::IdentityError;
use crate::Identity;
use core::time::Duration;
use ockam_core::{Address, AsyncTryClone, Result, Route};

impl Identity {
    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub async fn create_secure_channel_listener(
        &self,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<()> {
        let identity_clone = self.async_try_clone().await?;

        IdentityChannelListener::create(&self.ctx, address.into(), options.into(), identity_clone)
            .await?;

        Ok(())
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        let addresses = Addresses::generate(Role::Initiator);
        let options = options.into();

        let route = route.into();
        let next = route.next()?;
        options.setup_flow_control(&addresses, next)?;
        let access_control = options.create_access_control();

        DecryptorWorker::create_initiator(
            &self.ctx,
            route,
            identity_clone,
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
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
        timeout: Duration,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        let addresses = Addresses::generate(Role::Initiator);

        let route = route.into();
        let next = route.next()?;
        let options = options.into();
        options.setup_flow_control(&addresses, next)?;
        let access_control = options.create_access_control();

        DecryptorWorker::create_initiator(
            &self.ctx,
            route,
            identity_clone,
            addresses,
            options.trust_policy,
            access_control.decryptor_outgoing_access_control,
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
