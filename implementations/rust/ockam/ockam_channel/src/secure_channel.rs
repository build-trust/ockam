use crate::{
    KeyExchangeCompleted, SecureChannelListener, SecureChannelNewKeyExchanger, SecureChannelVault,
    SecureChannelWorker,
};
#[cfg(feature = "no_std")]
use ockam_core::compat::rand::random;
use ockam_core::{Address, Result, Route};
use ockam_key_exchange_core::KeyExchanger;
use ockam_node::Context;
#[cfg(feature = "std")]
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// SecureChannel info returned from start_initiator_channel
/// Auth hash can be used for further authentication of the channel
/// and tying it up cryptographically to some source of Trust. (e.g. Entities)
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SecureChannelInfo {
    worker_address: Address,
    auth_hash: [u8; 32],
}

impl SecureChannelInfo {
    /// Return a clone of the worker's address.
    pub fn address(&self) -> Address {
        self.worker_address.clone()
    }
    /// Return the auth hash.
    pub fn auth_hash(&self) -> [u8; 32] {
        self.auth_hash
    }
}

/// Secure Channel
pub struct SecureChannel;

impl SecureChannel {
    /// Create and start channel listener with given address using noise xx and software vault.
    #[cfg(all(feature = "software_vault", feature = "noise_xx"))]
    pub async fn create_listener<A: Into<Address>>(
        ctx: &Context,
        address: A,
        vault: &Address,
    ) -> Result<()> {
        use ockam_key_exchange_xx::XXNewKeyExchanger;
        use ockam_vault_sync_core::VaultSync;
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        let new_key_exchanger = XXNewKeyExchanger::new(vault.clone());
        Self::create_listener_extended(ctx, address, new_key_exchanger, vault).await
    }

    /// Create and start channel listener with given address.
    pub async fn create_listener_extended<
        A: Into<Address>,
        N: SecureChannelNewKeyExchanger,
        V: SecureChannelVault,
    >(
        ctx: &Context,
        address: A,
        new_key_exchanger: N,
        vault: V,
    ) -> Result<()> {
        let address = address.into();
        let channel_listener = SecureChannelListener::new(new_key_exchanger, vault);
        info!("Starting SecureChannel listener at {}", &address);
        ctx.start_worker(address, channel_listener).await?;

        Ok(())
    }

    /// Create initiator channel with given route to a remote channel listener using noise xx and software vault.
    #[cfg(all(feature = "software_vault", feature = "noise_xx"))]
    pub async fn create(
        ctx: &Context,
        route: impl Into<Route>,
        vault: &Address,
    ) -> Result<SecureChannelInfo> {
        use ockam_key_exchange_core::NewKeyExchanger;
        use ockam_key_exchange_xx::XXNewKeyExchanger;
        use ockam_vault_sync_core::VaultSync;
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        let new_key_exchanger = XXNewKeyExchanger::new(vault.clone());
        Self::create_extended(ctx, route, None, new_key_exchanger.initiator()?, vault).await
    }

    /// Create initiator channel with given route to a remote channel listener.
    pub async fn create_extended(
        ctx: &Context,
        route: impl Into<Route>,
        first_responder_address: Option<Address>,
        key_exchanger: impl KeyExchanger + Send + 'static,
        vault: impl SecureChannelVault + Send + 'static,
    ) -> Result<SecureChannelInfo> {
        let address_remote: Address = random();
        let address_local: Address = random();

        debug!(
            "Starting SecureChannel initiator at local: {}, remote: {}",
            &address_local, &address_remote
        );

        let route = route.into();

        let address: Address = random();
        let mut child_ctx = ctx.new_context(address).await?;
        let channel = SecureChannelWorker::new(
            true,
            route,
            address_remote.clone(),
            address_local.clone(),
            Some(child_ctx.address()),
            first_responder_address,
            key_exchanger,
            vault,
        )?;

        ctx.start_worker(vec![address_remote.clone(), address_local.clone()], channel)
            .await?;

        let resp = child_ctx
            .receive_timeout::<KeyExchangeCompleted>(120)
            .await?
            .take()
            .body();

        let info = SecureChannelInfo {
            worker_address: address_local,
            auth_hash: resp.auth_hash(),
        };

        Ok(info)
    }
}
