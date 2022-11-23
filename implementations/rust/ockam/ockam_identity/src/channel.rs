mod encryptor;
pub(crate) use encryptor::*;
mod decryptor;
pub(crate) use decryptor::*;
mod listener;
pub(crate) use listener::*;
mod messages;
pub(crate) use messages::*;
mod trust_policy;
use ockam_node::WorkerBuilder;
pub use trust_policy::*;
pub mod access_control;
mod local_info;
pub use local_info::*;

use crate::authenticated_storage::AuthenticatedStorage;
use crate::{Identity, IdentityVault};
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, AllowAll, AsyncTryClone, LocalDestinationOnly, Mailbox, Mailboxes, Result, Route,
};

impl<V: IdentityVault> Identity<V> {
    pub async fn create_secure_channel_listener(
        &self,
        address: impl Into<Address>,
        trust_policy: impl TrustPolicy,
        storage: &impl AuthenticatedStorage,
    ) -> Result<()> {
        let identity_clone = self.async_try_clone().await?;
        let storage_clone = storage.async_try_clone().await?;
        let listener = IdentityChannelListener::new(trust_policy, identity_clone, storage_clone);

        let mailbox = Mailbox::new(
            address.into(),
            Arc::new(AllowAll),
            Arc::new(LocalDestinationOnly), // Only talks to decryptors it creates
        );
        WorkerBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), listener)
            .start(&self.ctx)
            .await?;

        Ok(())
    }

    pub async fn create_secure_channel(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
        storage: &impl AuthenticatedStorage,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;
        let storage_clone = storage.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            storage_clone,
            Arc::new(trust_policy),
            Duration::from_secs(120),
        )
        .await
    }

    pub async fn create_secure_channel_extended(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
        storage: &impl AuthenticatedStorage,
        timeout: Duration,
    ) -> Result<Address> {
        let identity_clone = self.async_try_clone().await?;
        let storage_clone = storage.async_try_clone().await?;

        DecryptorWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            storage_clone,
            Arc::new(trust_policy),
            timeout,
        )
        .await
    }

    pub async fn stop_secure_channel(&self, channel: &Address) -> Result<()> {
        self.ctx.stop_worker(channel.clone()).await
    }
}
