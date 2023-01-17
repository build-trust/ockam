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
mod trust_policy;
pub use common::*;
pub use local_info::*;
pub use registry::*;
pub use trust_policy::*;

pub mod access_control;

use crate::authenticated_storage::AuthenticatedStorage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::channel::listener::IdentityChannelListener;
use crate::{Identity, IdentityVault};
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, AllowAll, AsyncTryClone, DenyAll, Result, Route};

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    pub async fn create_secure_channel_listener(
        &self,
        address: impl Into<Address>,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let identity_clone = self.async_try_clone().await?;

        let listener = IdentityChannelListener::new(trust_policy, identity_clone);

        self.ctx
            .start_worker(
                address.into(),
                listener,
                AllowAll, // TODO: @ac allow to customize
                DenyAll,
            )
            .await?;

        Ok(())
    }

    pub async fn create_secure_channel(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            Arc::new(trust_policy),
            Duration::from_secs(120),
        )
        .await
    }

    pub async fn create_secure_channel_extended(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
        timeout: Duration,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            Arc::new(trust_policy),
            timeout,
        )
        .await
    }

    pub async fn stop_secure_channel(&self, channel: &Address) -> Result<()> {
        self.ctx.stop_worker(channel.clone()).await
    }
}
