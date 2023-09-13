use crate::identity::models::Identifier;
use crate::identity::storage::Storage;
use crate::identity::{
    secure_channels, Credentials, CredentialsServer, Identities, IdentitiesCreation,
    IdentitiesKeys, IdentitiesRepository, Purpose, SecureChannel, SecureChannelListener,
    SecureChannelRegistry, SecureChannels, SecureChannelsBuilder,
};
use crate::identity::{Identity, SecureChannelListenerOptions, SecureChannelOptions};
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControls;
use ockam_core::{
    Address, IncomingAccessControl, Message, OutgoingAccessControl, Processor, Result, Route,
    Routed, Worker,
};
use ockam_node::{Context, HasContext, MessageReceiveOptions, MessageSendReceiveOptions};
use ockam_vault::{Vault, VaultStorage};

use crate::remote::{RemoteForwarder, RemoteForwarderInfo, RemoteForwarderOptions};
use crate::stream::Stream;
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
/// use ockam_vault::storage::PersistentStorage;
///
/// async fn make_node(ctx: Context) -> Result<Node> {
///   let node = Node::builder().with_vault_storage(PersistentStorage::create(Path::new("vault")).await?).build(ctx).await?;
///   Ok(node)
/// }
///
///
/// ```
/// Here is another example where we specify a local LMDB database to store identity attributes
/// ```rust
/// use std::sync::Arc;
/// use ockam::{Node, Result};
/// use ockam::LmdbStorage;
/// use ockam_node::Context;
///
/// async fn make_node(ctx: Context) -> Result<Node> {
///    let lmdb_storage = Arc::new(LmdbStorage::new("identities").await?);
///    let node = Node::builder().with_identities_storage(lmdb_storage).build(ctx).await?;
///    Ok(node)
/// }
/// ```
pub fn node(ctx: Context) -> Node {
    Node {
        context: ctx,
        secure_channels: secure_channels(),
    }
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

    /// Create a new stream
    pub async fn create_stream(&self) -> Result<Stream> {
        Stream::new(self.get_context()).await
    }

    /// Create a new forwarder
    pub async fn create_forwarder(
        &self,
        orchestrator_route: impl Into<Route>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        RemoteForwarder::create(self.get_context(), orchestrator_route, options).await
    }

    /// Create a new static forwarder
    pub async fn create_static_forwarder(
        &self,
        orchestrator_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        RemoteForwarder::create_static(self.get_context(), orchestrator_route, alias, options).await
    }

    /// Create an Identity
    pub async fn create_identity(&self) -> Result<Identifier> {
        Ok(self
            .identities_creation()
            .create_identity()
            .await?
            .identifier()
            .clone())
    }

    /// Create the [`SecureChannel`] [`PurposeKey`]
    pub async fn create_secure_channel_key(&self, identifier: &Identifier) -> Result<()> {
        let _ = self
            .identities()
            .purpose_keys()
            .create_purpose_key(identifier, Purpose::SecureChannel)
            .await?;

        Ok(())
    }

    /// Import an Identity given its private key and change history
    /// Note: the data is not persisted!
    pub async fn import_private_identity(
        &self,
        identity_history: &str,
        secret: &str,
    ) -> Result<Identity> {
        self.identities_creation()
            .import_private_identity(identity_history, secret)
            .await
    }

    /// Import an Identity given that was exported as a hex-encoded string
    pub async fn import_identity_hex(&self, data: &str) -> Result<Identity> {
        self.identities_creation()
            .import(
                None,
                &hex::decode(data).map_err(|_| OckamError::InvalidHex)?,
            )
            .await
    }

    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub async fn create_secure_channel_listener(
        &self,
        identifier: &Identifier,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<SecureChannelListener> {
        self.secure_channels()
            .create_secure_channel_listener(self.get_context(), identifier, address, options)
            .await
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
    pub async fn start_worker<W>(&self, address: impl Into<Address>, worker: W) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        self.context.start_worker(address, worker).await
    }

    /// Start a new worker instance at the given address with given Access Controls
    pub async fn start_worker_with_access_control<W>(
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
            .await
    }

    /// Start a new processor instance at the given address. Default Access Control is DenyAll
    pub async fn start_processor<P>(&self, address: impl Into<Address>, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        self.context.start_processor(address, processor).await
    }

    /// Start a new processor instance at the given address with given Access Controls
    pub async fn start_processor_with_access_control<P>(
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
            .await
    }

    /// Signal to the local runtime to shut down
    pub async fn stop(&mut self) -> Result<()> {
        self.context.stop().await
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
    ) -> Result<Routed<M>>
    where
        M: Message,
    {
        self.context.receive_extended(options).await
    }

    /// Return secure channel services
    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
    }

    /// Return services to manage identities
    pub fn identities(&self) -> Arc<Identities> {
        self.secure_channels().identities()
    }

    /// Return services to create and import identities
    pub fn identities_creation(&self) -> Arc<IdentitiesCreation> {
        self.identities().identities_creation()
    }

    /// Return services to manage identities keys
    pub fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        self.identities().identities_keys()
    }

    /// Return services to manage credentials
    pub fn credentials(&self) -> Arc<Credentials> {
        self.identities().credentials()
    }

    /// Return services to serve credentials
    pub fn credentials_server(&self) -> Arc<dyn CredentialsServer> {
        self.identities().credentials_server()
    }

    /// Return the repository used to store identities data
    pub fn repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities().repository()
    }

    /// Return a new builder for top-level services
    pub fn builder() -> NodeBuilder {
        NodeBuilder::new()
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
    fn new() -> Self {
        Self {
            builder: SecureChannels::builder(),
        }
    }

    /// Set [`Vault`]
    pub fn with_vault(mut self, vault: Vault) -> Self {
        self.builder = self.builder.with_vault(vault);
        self
    }

    /// With Software Vault with given Storage
    pub fn with_vault_storage(mut self, storage: VaultStorage) -> Self {
        self.builder = self.builder.with_vault_storage(storage);
        self
    }

    /// Set a specific storage for identities
    pub fn with_identities_storage(mut self, storage: Arc<dyn Storage>) -> Self {
        self.builder = self.builder.with_identities_storage(storage);
        self
    }

    /// Set a specific identities repository
    pub fn with_identities_repository(mut self, repository: Arc<dyn IdentitiesRepository>) -> Self {
        self.builder = self.builder.with_identities_repository(repository);
        self
    }

    /// Set a specific secure channels registry
    pub fn with_secure_channels_registry(mut self, registry: SecureChannelRegistry) -> Self {
        self.builder = self.builder.with_secure_channels_registry(registry);
        self
    }

    /// Build top level services
    pub async fn build(self, ctx: Context) -> Result<Node> {
        Ok(Node {
            context: ctx,
            secure_channels: self.builder.build(),
        })
    }
}
