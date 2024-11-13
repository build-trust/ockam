use core::sync::atomic::AtomicBool;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControls;
use ockam_core::Result;
use ockam_core::{Address, Route};
use ockam_node::{Context, WorkerBuilder};
use tracing::info;

use crate::identities::Identities;
use crate::models::Identifier;
use crate::secure_channel::handshake_worker::HandshakeWorker;
use crate::secure_channel::{
    Addresses, DecryptorHandler, RemoteRoute, Role, SecureChannelListenerOptions,
    SecureChannelListenerWorker, SecureChannelOptions, SecureChannelRegistry,
    SecureChannelSharedState,
};
#[cfg(feature = "storage")]
use crate::SecureChannelsBuilder;
use crate::{
    IdentityError, SecureChannel, SecureChannelListener, SecureChannelRegistryEntry,
    SecureChannelRepository, Vault,
};

/// Identity implementation
#[derive(Clone)]
pub struct SecureChannels {
    pub(crate) identities: Arc<Identities>,
    pub(crate) secure_channel_registry: SecureChannelRegistry,
    pub(crate) secure_channel_repository: Arc<dyn SecureChannelRepository>,
}

impl SecureChannels {
    /// Constructor
    pub fn new(
        identities: Arc<Identities>,
        secure_channel_registry: SecureChannelRegistry,
        secure_channel_repository: Arc<dyn SecureChannelRepository>,
    ) -> Self {
        Self {
            identities,
            secure_channel_registry,
            secure_channel_repository,
        }
    }

    /// Constructor
    pub fn from_identities(
        identities: Arc<Identities>,
        secure_channel_repository: Arc<dyn SecureChannelRepository>,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            identities,
            SecureChannelRegistry::default(),
            secure_channel_repository,
        ))
    }

    /// Return the identities services associated to this service
    pub fn identities(&self) -> Arc<Identities> {
        self.identities.clone()
    }

    /// Vault
    pub fn vault(&self) -> Vault {
        self.identities.vault()
    }

    /// Return the secure channel registry
    pub fn secure_channel_registry(&self) -> SecureChannelRegistry {
        self.secure_channel_registry.clone()
    }

    /// Return the secure channel repository
    pub fn secure_channel_repository(&self) -> Arc<dyn SecureChannelRepository> {
        self.secure_channel_repository.clone()
    }

    /// Create a builder for secure channels
    #[cfg(feature = "storage")]
    pub async fn builder() -> Result<SecureChannelsBuilder> {
        Ok(SecureChannelsBuilder {
            identities_builder: Identities::builder().await?,
            registry: SecureChannelRegistry::new(),
            secure_channel_repository: Arc::new(crate::SecureChannelSqlxDatabase::create().await?),
        })
    }
}

impl SecureChannels {
    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub fn create_secure_channel_listener(
        &self,
        ctx: &Context,
        identifier: &Identifier,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<SecureChannelListener> {
        let address = address.into();
        let options = options.into();
        let key_exchange_only = options.key_exchange_only;
        let flow_control_id = options.flow_control_id.clone();

        SecureChannelListenerWorker::create(
            ctx,
            Arc::new(self.clone()),
            identifier,
            address.clone(),
            options,
        )?;

        Ok(SecureChannelListener::new(
            address,
            key_exchange_only,
            flow_control_id,
        ))
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
        identifier: &Identifier,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<SecureChannel> {
        let addresses = Addresses::generate(Role::Initiator);
        let options = options.into();
        let flow_control_id = options.flow_control_id.clone();

        let route = route.into();
        let next = route.next()?;
        options.setup_flow_control(ctx.flow_controls(), &addresses, next);
        let decryptor_outgoing_access_control =
            options.create_decryptor_outgoing_access_control(ctx.flow_controls());

        // TODO: Allow manual PurposeKey management
        let purpose_key = self
            .identities
            .purpose_keys()
            .purpose_keys_creation()
            .get_or_create_secure_channel_purpose_key(identifier)
            .await?;

        let credential_retriever = match &options.credential_retriever_creator {
            Some(credential_retriever_creator) => {
                let credential_retriever = credential_retriever_creator.create(identifier).await?;
                credential_retriever.initialize().await?;
                Some(credential_retriever)
            }
            None => None,
        };

        let secure_channel_repository = if options.is_persistent {
            Some(self.secure_channel_repository())
        } else {
            None
        };

        let encryptor_remote_route = RemoteRoute::create();
        let Some(their_identifier) = HandshakeWorker::create(
            ctx,
            Arc::new(self.clone()),
            addresses.clone(),
            identifier.clone(),
            purpose_key,
            options.trust_policy,
            decryptor_outgoing_access_control,
            credential_retriever,
            options.authority,
            Some(route),
            Some(options.timeout),
            Role::Initiator,
            options.key_exchange_only,
            secure_channel_repository,
            encryptor_remote_route.clone(),
        )
        .await?
        else {
            return Err(IdentityError::HandshakeInternalError)?;
        };

        Ok(SecureChannel::new(
            ctx.flow_controls().clone(),
            their_identifier,
            encryptor_remote_route,
            addresses,
            options.key_exchange_only,
            flow_control_id,
        ))
    }

    /// Start a decryptor side for a previously existed and persisted secure channel
    /// Only decryptor api part is started
    pub async fn start_persisted_secure_channel_decryptor(
        &self,
        ctx: &Context,
        decryptor_remote_address: &Address,
    ) -> Result<SecureChannel> {
        info!(
            "Starting persisted secure channel: {}",
            decryptor_remote_address
        );

        let Some(persisted_secure_channel) = self
            .secure_channel_repository
            .get(decryptor_remote_address)
            .await?
        else {
            return Err(IdentityError::PersistentSecureChannelNotFound)?;
        };

        let decryption_key = persisted_secure_channel.decryption_key_handle().clone();

        self.vault()
            .secure_channel_vault
            .load_aead_key(&decryption_key)
            .await?;

        let my_identifier = persisted_secure_channel.my_identifier();
        let their_identifier = persisted_secure_channel.their_identifier();
        let role = persisted_secure_channel.role();
        let shared_state = SecureChannelSharedState {
            remote_route: RemoteRoute::create(),                 // Unused
            should_send_close: Arc::new(AtomicBool::new(false)), // Don't need to send anything
        };

        let mut addresses = Addresses::generate(role);
        // FIXME: All other addresses except these two are random and incorrect, we don't use them
        //  for now though
        addresses.decryptor_remote = persisted_secure_channel.decryptor_remote().clone();
        addresses.decryptor_api = persisted_secure_channel.decryptor_api().clone();

        let decryptor_handler = DecryptorHandler::new(
            self.identities(),
            None, // We don't need authority, we won't verify any credentials
            role,
            true,
            addresses.clone(),
            decryption_key,
            self.vault().secure_channel_vault.clone(),
            their_identifier.clone(),
            shared_state.clone(),
        );

        let decryptor_worker = HandshakeWorker::new(
            Arc::new(self.clone()),
            None, // No callback will happen
            None,
            my_identifier.clone(),
            addresses.clone(),
            role,
            true,
            None, // No remote interaction
            Some(decryptor_handler),
            None, // We don't need authority, we won't verify any credentials
            self.identities.change_history_repository(),
            None, // We don't need credential retriever, we won't present any credentials
            // Key exchange only secure channel's state is unchanged after the initial creation, so no need to update it
            None,
            shared_state.clone(),
        );

        WorkerBuilder::new(decryptor_worker)
            .with_address(addresses.decryptor_api.clone()) // We only need API address here
            .start(ctx)?;

        let sc = SecureChannel::new(
            ctx.flow_controls().clone(),
            their_identifier.clone(),
            shared_state.remote_route,
            addresses.clone(),
            true,
            FlowControls::generate_flow_control_id(), // This is random and doesn't matter
        );

        let info = SecureChannelRegistryEntry::new(
            addresses.encryptor.clone(),
            addresses.encryptor_api.clone(),
            addresses.decryptor_remote.clone(),
            addresses.decryptor_api.clone(),
            role.is_initiator(),
            my_identifier.clone(),
            their_identifier.clone(),
            Address::random_local(), // Random, unused for now
        );

        self.secure_channel_registry.register_channel(info)?;

        Ok(sc)
    }

    /// Stop a SecureChannel given an encryptor address
    pub fn stop_secure_channel(&self, ctx: &Context, channel: &Address) -> Result<()> {
        ctx.stop_address(channel)
    }
}
