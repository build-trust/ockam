use crate::{
    KeyExchangeCompleted, SecureChannelDecryptor, SecureChannelKeyExchanger, SecureChannelListener,
    SecureChannelNewKeyExchanger, SecureChannelVault,
};
use ockam_core::compat::{sync::Arc, vec::Vec};
use ockam_core::{Address, Mailbox, Mailboxes, Result, Route};
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
        ctx.start_worker(address, channel_listener).await?;

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
        let route = route.into();

        // TODO @ac - does have random() have different entropy to random_local?
        // let callback_address: Address = random();
        let callback_address =
            Address::random_tagged("SecureChannel.initiator.callback_address.detached");

        //let mut child_ctx = ctx.new_detached(callback_address.clone()).await?;

        // TODO @ac 0#SecureChannel.initiator.callback_address.detached
        // in:
        // out:
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                callback_address.clone(),
                Arc::new(ockam_core::ToDoAccessControl),
                Arc::new(ockam_core::ToDoAccessControl),
            ),
            vec![],
        );
        let mut child_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;

        let decryptor = SecureChannelDecryptor::new_initiator(
            key_exchanger,
            Some(callback_address),
            route,
            custom_payload,
            vault.async_try_clone().await?,
        )
        .await?;

        // TODO @ac - does have random() have different entropy to random_local?
        // let address_remote: Address = random();
        let address_remote = Address::random_tagged("SecureChannel.initiator");

        debug!(
            "Starting SecureChannel initiator at remote: {}",
            &address_remote
        );

        // ctx.start_worker(address_remote.clone(), decryptor).await?;

        // TODO @ac
        // const TCP: ockam_core::TransportType = ockam_core::TransportType::new(1);

        // TODO @ac 0#SecureChannel.initiator
        // in:
        // out:
        let mailbox = Mailbox::new(
            address_remote,
            // TODO @ac need a way to specify AC for incoming from client API because we
            //          don't know if this is coming in over Transport or LocalOrigin or...
            // Arc::new(ockam_core::AnyAccessControl::new(
            //     ockam_node::access_control::AllowTransport::single(TCP),
            //     LocalOriginOnly, // TODO @ac ???
            // )),
            // Arc::new(LocalOriginOnly), // TODO @ac ???
            Arc::new(ockam_core::ToDoAccessControl), // TODO @ac ???
            Arc::new(ockam_core::ToDoAccessControl), // TODO @ac ???
        );
        WorkerBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), decryptor)
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
