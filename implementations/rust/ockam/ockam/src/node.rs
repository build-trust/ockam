use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControls;
use ockam_core::{
    Address, IncomingAccessControl, Message, OutgoingAccessControl, Processor, Result, Route,
    Routed, TryClone, Worker,
};
use ockam_identity::{
    CredentialRepository, IdentitiesAttributes, IdentitiesVerification,
    IdentityAttributesRepository, PurposeKeys, Vault,
};
use ockam_node::{Context, HasContext, MessageReceiveOptions, MessageSendReceiveOptions};
use ockam_vault::storage::SecretsRepository;
use ockam_vault::SigningSecretKeyHandle;

use crate::identity::models::Identifier;
#[cfg(feature = "storage")]
use crate::identity::secure_channels;
use crate::identity::{
    ChangeHistoryRepository, Credentials, Identities, IdentitiesCreation, IdentitiesKeys,
    SecureChannel, SecureChannelListener, SecureChannelRegistry, SecureChannels,
    SecureChannelsBuilder,
};
use crate::identity::{SecureChannelListenerOptions, SecureChannelOptions};
use crate::remote::{RemoteRelay, RemoteRelayInfo, RemoteRelayOptions};
use crate::OckamError;

/// This struct supports all the ockam services for managing identities
/// and creating secure channels
pub struct Node {
    context: Context,
    secure_channels: Arc<SecureChannels>,
}

/// Create a default node (with no persistence)
/// Persistent implementations are available by using a builder.
/// For example, you can use a FileStorage backend to support the node vault.
/// ```rust
/// use std::path::Path;
/// use std::sync::Arc;
/// use ockam::{Node, Result};
/// use ockam_node::Context;
/// use ockam_vault::storage::SecretsSqlxDatabase;
///
/// async fn make_node(ctx: Context) -> Result<Node> {
///   let node = Node::builder()
///       .await?
///       .with_secrets_repository(Arc::new(SecretsSqlxDatabase::create().await?))
///       .build(&ctx)?;
///   Ok(node)
/// }
///
///
/// ```
#[cfg(feature = "storage")]
pub async fn node(ctx: Context) -> Result<Node> {
    Ok(Node {
        context: ctx,
        secure_channels: secure_channels().await?,
    })
}

impl Node {
    /// Return the node's [`FlowControls`]
    pub fn flow_controls(&self) -> &FlowControls {
        self.context.flow_controls()
    }

    /// Return the current context
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Create a new relay
    pub async fn create_relay(
        &self,
        orchestrator_route: impl Into<Route>,
        options: RemoteRelayOptions,
    ) -> Result<RemoteRelayInfo> {
        RemoteRelay::create(self.get_context(), orchestrator_route, options).await
    }

    /// Create a new static relay
    pub async fn create_static_relay(
        &self,
        orchestrator_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteRelayOptions,
    ) -> Result<RemoteRelayInfo> {
        RemoteRelay::create_static(self.get_context(), orchestrator_route, alias, options).await
    }

    /// Create an Identity
    pub async fn create_identity(&self) -> Result<Identifier> {
        self.identities_creation().create_identity().await
    }

    /// Create the [`SecureChannel`] [`PurposeKey`]
    pub async fn create_secure_channel_key(&self, identifier: &Identifier) -> Result<()> {
        let _ = self
            .identities()
            .purpose_keys()
            .purpose_keys_creation()
            .create_secure_channel_purpose_key(identifier)
            .await?;

        Ok(())
    }

    /// Import an Identity given its private key and change history
    pub async fn import_private_identity(
        &self,
        expected_identifier: Option<&Identifier>,
        identity_change_history: &[u8],
        key: &SigningSecretKeyHandle,
    ) -> Result<Identifier> {
        self.identities_creation()
            .import_private_identity(expected_identifier, identity_change_history, key)
            .await
    }

    /// Import an Identity given that was exported as a hex-encoded string
    pub async fn import_identity_hex(
        &self,
        expected_identifier: Option<&Identifier>,
        data: &str,
    ) -> Result<Identifier> {
        self.identities_verification()
            .import(
                expected_identifier,
                &hex::decode(data).map_err(|_| OckamError::InvalidHex)?,
            )
            .await
    }

    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub fn create_secure_channel_listener(
        &self,
        identifier: &Identifier,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<SecureChannelListener> {
        self.secure_channels().create_secure_channel_listener(
            self.get_context(),
            identifier,
            address,
            options,
        )
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        identifier: &Identifier,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<SecureChannel> {
        self.secure_channels()
            .create_secure_channel(self.get_context(), identifier, route, options)
            .await
    }

    /// Start a new worker instance at the given address. Default Access Control is AllowAll
    pub fn start_worker<W>(&self, address: impl Into<Address>, worker: W) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        self.context.start_worker(address, worker)
    }

    /// Start a new worker instance at the given address with given Access Controls
    pub fn start_worker_with_access_control<W>(
        &self,
        address: impl Into<Address>,
        worker: W,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        self.context
            .start_worker_with_access_control(address, worker, incoming, outgoing)
    }

    /// Start a new processor instance at the given address. Default Access Control is DenyAll
    pub fn start_processor<P>(&self, address: impl Into<Address>, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        self.context.start_processor(address, processor)
    }

    /// Start a new processor instance at the given address with given Access Controls
    pub fn start_processor_with_access_control<P>(
        &self,
        address: impl Into<Address>,
        processor: P,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        self.context
            .start_processor_with_access_control(address, processor, incoming, outgoing)
    }

    /// Signal to the local runtime to shut down
    pub async fn shutdown(&mut self) -> Result<()> {
        self.context.shutdown_node().await
    }

    /// Send a message to an address or via a fully-qualified route
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.context.send(route, msg).await
    }

    /// Send a message to an address or via a fully-qualified route and receive a response
    pub async fn send_and_receive<M>(&self, route: impl Into<Route>, msg: impl Message) -> Result<M>
    where
        M: Message,
    {
        self.context.send_and_receive(route, msg).await
    }

    /// Send a message to an address or via a fully-qualified route and receive a response
    pub async fn send_and_receive_extended<M>(
        &self,
        route: impl Into<Route>,
        msg: impl Message,
        options: MessageSendReceiveOptions,
    ) -> Result<Routed<M>>
    where
        M: Message,
    {
        self.context
            .send_and_receive_extended(route, msg, options)
            .await
    }

    /// Send a message to an address or via a fully-qualified route and receive a response
    pub async fn receive<M: Message>(&mut self) -> Result<Routed<M>> {
        self.context.receive::<M>().await
    }

    /// Send a message to an address or via a fully-qualified route and receive a response
    pub async fn receive_extended<M: Message>(
        &mut self,
        options: MessageReceiveOptions,
    ) -> Result<Routed<M>> {
        self.context.receive_extended(options).await
    }

    /// Return secure channel services
    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
    }

    /// Return services to manage identities
    pub fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    /// Return services to create and import identities
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        self.secure_channels.identities().identities_creation()
    }

    /// Return services to create and import identities
    pub fn identities_verification(&self) -> Arc<IdentitiesVerification> {
        self.secure_channels.identities().identities_verification()
    }

    /// Return services to manage identities keys
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        self.secure_channels.identities().identities_keys()
    }

    /// Return services to manage credentials
    pub fn credentials(&self) -> Arc<Credentials> {
        self.secure_channels.identities().credentials()
    }

    /// Return the [`Vault`]
    pub fn vault(&self) -> Vault {
        self.secure_channels.vault()
    }

    /// Return the vault used by secure channels
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        self.secure_channels.identities().purpose_keys()
    }

    /// Return the repository used to store identities data
    pub fn identities_repository(&self) -> Arc<dyn ChangeHistoryRepository> {
        self.secure_channels
            .identities()
            .change_history_repository()
    }

    /// Return the service responsible for managing identities attributes
    pub fn identities_attributes(&self) -> Arc<IdentitiesAttributes> {
        self.secure_channels.identities().identities_attributes()
    }

    /// Return a new builder for top-level services
    #[cfg(feature = "storage")]
    pub async fn builder() -> Result<NodeBuilder> {
        NodeBuilder::new().await
    }
}

/// This trait can be used to integrate transports into a node
impl HasContext for Node {
    /// Return a context
    fn get_context(&self) -> &Context {
        self.context()
    }
}

/// Builder for top level services
/// It merely encapsulates a secure channel builder for now
#[derive(Clone)]
pub struct NodeBuilder {
    builder: SecureChannelsBuilder,
}

impl NodeBuilder {
    #[cfg(feature = "storage")]
    async fn new() -> Result<Self> {
        Ok(Self {
            builder: SecureChannels::builder().await?,
        })
    }

    /// Set [`Vault`]
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.builder = self.builder.with_vault(vault);
        self
    }

    /// With Software Vault with given secrets repository
    pub fn with_secrets_repository(mut self, repository: Arc<dyn SecretsRepository>) -> Self {
        self.builder = self.builder.with_secrets_repository(repository);
        self
    }

    /// Set a specific change history repository
    pub fn with_change_history_repository(
        mut self,
        repository: Arc<dyn ChangeHistoryRepository>,
    ) -> Self {
        self.builder = self.builder.with_change_history_repository(repository);
        self
    }

    /// Set a specific identity attributes repository
    pub fn with_identity_attributes_repository(
        mut self,
        repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        self.builder = self.builder.with_identity_attributes_repository(repository);
        self
    }

    /// Set a specific cached credential repository
    pub fn with_cached_credential_repository(
        mut self,
        repository: Arc<dyn CredentialRepository>,
    ) -> Self {
        self.builder = self.builder.with_cached_credential_repository(repository);
        self
    }

    /// Set a specific secure channels registry
    pub fn with_secure_channels_registry(mut self, registry: SecureChannelRegistry) -> Self {
        self.builder = self.builder.with_secure_channels_registry(registry);
        self
    }

    /// Build top level services
    pub fn build(self, ctx: &Context) -> Result<Node> {
        Ok(Node {
            context: ctx.try_clone()?,
            secure_channels: self.builder.build(),
        })
    }
}
