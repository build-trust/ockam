use crate::{
    KeyExchangeCompleted, SecureChannelDecryptor, SecureChannelKeyExchanger, SecureChannelListener,
    SecureChannelNewKeyExchanger, SecureChannelVault,
};
use ockam_core::compat::{sync::Arc, vec::Vec};
use ockam_core::{
    AccessControl, Address, AllowAll, AllowOnwardAddresses, AllowSourceAddress, DenyAll,
    LocalOnwardOnly, Mailbox, Mailboxes, Result, Route,
};
use ockam_node::{Context, WorkerBuilder};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// SecureChannel info returned from start_initiator_channel
/// Auth hash can be used for further authentication of the channel
/// and tying it up cryptographically to some source of Trust. (e.g. Entities)
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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
    pub async fn create_listener<A: Into<Address>, V: SecureChannelVault>(
        ctx: &Context,
        address: A,
        vault: &V,
    ) -> Result<()> {
        use ockam_key_exchange_xx::XXNewKeyExchanger;
        let new_key_exchanger = XXNewKeyExchanger::new(vault.async_try_clone().await?);
        Self::create_listener_extended(
            ctx,
            address,
            new_key_exchanger,
            vault.async_try_clone().await?,
        )
        .await
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
        let mailboxes = Mailboxes::main(address, Arc::new(AllowAll), Arc::new(DenyAll));
        WorkerBuilder::with_mailboxes(mailboxes, channel_listener)
            .start(ctx)
            .await?;

        Ok(())
    }

    /// Create initiator channel with given route to a remote channel listener using noise xx and software vault.
    #[cfg(all(feature = "software_vault", feature = "noise_xx"))]
    pub async fn create<V: SecureChannelVault>(
        ctx: &Context,
        route: impl Into<Route>,
        vault: &V,
    ) -> Result<SecureChannelInfo> {
        use ockam_key_exchange_core::NewKeyExchanger;
        use ockam_key_exchange_xx::XXNewKeyExchanger;
        let new_key_exchanger = XXNewKeyExchanger::new(vault.async_try_clone().await?);
        Self::create_extended(
            ctx,
            route,
            None,
            new_key_exchanger.initiator().await?,
            vault.async_try_clone().await?,
        )
        .await
    }

    /// Create initiator channel with given route to a remote channel listener.
    pub async fn create_extended(
        ctx: &Context,
        route: impl Into<Route>,
        custom_payload: Option<Vec<u8>>,
        key_exchanger: impl SecureChannelKeyExchanger,
        vault: impl SecureChannelVault,
    ) -> Result<SecureChannelInfo> {
        Self::create_extended_wrapped(ctx, route, custom_payload, key_exchanger, vault, None, None)
            .await
    }

    /// Create initiator channel with given route to a remote channel listener.
    /// Used while this channel is wrapped inside an Identity Secure Channel
    pub async fn create_extended_wrapped(
        ctx: &Context,
        route: impl Into<Route>,
        custom_payload: Option<Vec<u8>>,
        key_exchanger: impl SecureChannelKeyExchanger,
        vault: impl SecureChannelVault,
        wrapped_outgoing_address: Option<Address>,
        address_internal: Option<Address>,
    ) -> Result<SecureChannelInfo> {
        let route = route.into();

        let address_remote = Address::random_tagged("SecureChannel.initiator.decryptor.remote");
        let address_internal = address_internal.unwrap_or_else(|| {
            Address::random_tagged("SecureChannel.initiator.decryptor.internal")
        });
        let callback_address =
            Address::random_tagged("SecureChannel.initiator.callback_address.detached");

        let mailboxes = Mailboxes::new(
            Mailbox::new(
                callback_address.clone(),
                Arc::new(AllowSourceAddress(address_internal.clone())), // Allow only from the Decryptor
                Arc::new(DenyAll),
            ),
            vec![],
        );
        let mut child_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;

        let decryptor = SecureChannelDecryptor::new_initiator(
            key_exchanger,
            address_remote.clone(),
            address_internal.clone(),
            Some(callback_address.clone()),
            route,
            custom_payload,
            vault.async_try_clone().await?,
            vec![],
        )
        .await?;

        debug!(
            "Starting SecureChannel initiator at remote: {}",
            &address_remote
        );

        let remote_mailbox = Mailbox::new(
            address_remote,
            // Doesn't matter since we check incoming messages cryptographically,
            // but this may be reduced to allowing only from the transport connection that was used
            // to create this channel initially
            Arc::new(AllowAll),
            // Communicate to the other side of the channel
            Arc::new(AllowAll),
        );
        let outgoing_access_control: Arc<dyn AccessControl> = match wrapped_outgoing_address {
            // FIXME: @ac Also deny to other secure channels
            None => Arc::new(LocalOnwardOnly), // Prevent exploit of using our node as an authorized proxy
            Some(outgoing_address) => Arc::new(AllowOnwardAddresses(vec![
                outgoing_address,
                callback_address,
            ])),
        };
        let internal_mailbox =
            Mailbox::new(address_internal, Arc::new(DenyAll), outgoing_access_control);
        WorkerBuilder::with_mailboxes(
            Mailboxes::new(remote_mailbox, vec![internal_mailbox]),
            decryptor,
        )
        .start(ctx)
        .await?;

        let resp = child_ctx
            .receive_timeout::<KeyExchangeCompleted>(120)
            .await?
            .take()
            .body();

        let info = SecureChannelInfo {
            worker_address: resp.address().clone(),
            auth_hash: resp.auth_hash(),
        };

        Ok(info)
    }
}
