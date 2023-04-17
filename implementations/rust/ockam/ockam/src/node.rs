use crate::remote::{RemoteForwarder, RemoteForwarderInfo, RemoteForwarderOptions};
use crate::stream::Stream;
use core::time::Duration;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, AsyncTryClone, IncomingAccessControl, Message, OutgoingAccessControl, Processor,
    Result, Route, Routed, Worker,
};
use ockam_identity::{
    secure_channels, Credentials, CredentialsServer, Identities, IdentitiesCreation,
    IdentitiesKeys, IdentitiesRepository, SecureChannelRegistry, SecureChannels,
    SecureChannelsBuilder, Storage,
};
use ockam_identity::{
    IdentitiesVault, Identity, SecureChannelListenerOptions, SecureChannelOptions,
};
use ockam_node::{Context, MessageReceiveOptions, MessageSendReceiveOptions};
#[cfg(feature = "std")]
use ockam_transport_tcp::TcpTransport;

/// This struct supports all the ockam services for managing identities
/// and creating secure channels
pub struct Node {
    context: Context,
    secure_channels: Arc<SecureChannels>,
}

/// Create a default node (with no persistence)
pub fn node(ctx: Context) -> Node {
    Node {
        context: ctx,
        secure_channels: secure_channels(),
    }
}

impl Node {
    /// Return the current context
    pub async fn context(&self) -> Result<Context> {
        self.context.async_try_clone().await
    }

    /// Create and return a TCP transport
    #[cfg(feature = "std")]
    pub async fn create_tcp_transport(&self) -> Result<TcpTransport> {
        TcpTransport::create(&self.context).await
    }

    /// Create a new stream
    pub async fn create_stream(&self) -> Result<Stream> {
        Stream::new(&self.context().await?).await
    }

    /// Create a new forwarder
    pub async fn create_forwarder(
        &self,
        orchestrator_route: impl Into<Route>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        RemoteForwarder::create(&self.context().await?, orchestrator_route, options).await
    }

    /// Create a new static forwarder
    pub async fn create_static_forwarder(
        &self,
        orchestrator_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        RemoteForwarder::create_static(&self.context().await?, orchestrator_route, alias, options)
            .await
    }

    /// Create an Identity
    pub async fn create_identity(&self) -> Result<Identity> {
        self.identities_creation().create_identity().await
    }

    /// Import an Identity given its private key and change history
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
        self.identities_creation().import_identity_hex(data).await
    }

    /// Spawns a SecureChannel listener at given `Address` with given [`SecureChannelListenerOptions`]
    pub async fn create_secure_channel_listener(
        &self,
        identity: &Identity,
        address: impl Into<Address>,
        options: impl Into<SecureChannelListenerOptions>,
    ) -> Result<()> {
        self.secure_channels()
            .create_secure_channel_listener(&self.context, identity, address, options)
            .await
    }

    /// Initiate a SecureChannel using `Route` to the SecureChannel listener and [`SecureChannelOptions`]
    pub async fn create_secure_channel(
        &self,
        identity: &Identity,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
    ) -> Result<Address> {
        self.secure_channels()
            .create_secure_channel(&self.context, identity, route, options)
            .await
    }

    /// Extended function to create a SecureChannel with [`SecureChannelOptions`]
    pub async fn create_secure_channel_extended(
        &self,
        identity: &Identity,
        route: impl Into<Route>,
        options: impl Into<SecureChannelOptions>,
        timeout: Duration,
    ) -> Result<Address> {
        self.secure_channels()
            .create_secure_channel_extended(&self.context, identity, route, options, timeout)
            .await
    }

    /// Start a new worker instance at the given address
    pub async fn start_worker<NM, NW>(
        &self,
        address: impl Into<Address>,
        worker: NW,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        self.context
            .start_worker(address, worker, incoming, outgoing)
            .await
    }

    /// Start a new processor instance at the given address
    pub async fn start_processor<P>(
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
            .start_processor(address, processor, incoming, outgoing)
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
    pub fn credentials(&self) -> Arc<dyn Credentials> {
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

    /// Return the vault used by secure channels
    pub fn secure_channels_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.secure_channels().vault()
    }

    /// Return the vault used by identities
    pub fn identities_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.identities().vault()
    }

    /// Return a new builder for top-level services
    pub fn builder() -> NodeBuilder {
        NodeBuilder::new()
    }
}

/// Builder for top level services
/// It merely encapsulates a secure channel builder for now
#[derive(Clone)]
pub struct NodeBuilder {
    builder: SecureChannelsBuilder,
}

impl NodeBuilder {
    fn new() -> NodeBuilder {
        NodeBuilder {
            builder: SecureChannels::builder(),
        }
    }

    /// Set a specific vault storage for identities and secure channels
    pub fn with_vault_storage(
        &mut self,
        storage: Arc<dyn ockam_core::vault::storage::Storage>,
    ) -> NodeBuilder {
        self.builder = self.builder.with_vault_storage(storage);
        self.clone()
    }

    /// Set a specific identities vault
    pub fn with_identities_vault(&mut self, vault: Arc<dyn IdentitiesVault>) -> NodeBuilder {
        self.builder = self.builder.with_identities_vault(vault);
        self.clone()
    }

    /// Set a specific storage for identities
    pub fn with_identities_storage(&mut self, storage: Arc<dyn Storage>) -> NodeBuilder {
        self.builder = self.builder.with_identities_storage(storage);
        self.clone()
    }

    /// Set a specific identities repository
    pub fn with_identities_repository(
        &mut self,
        repository: Arc<dyn IdentitiesRepository>,
    ) -> NodeBuilder {
        self.builder = self.builder.with_identities_repository(repository);
        self.clone()
    }

    /// Set a specific secure channels registry
    pub fn with_secure_channels_registry(
        &mut self,
        registry: SecureChannelRegistry,
    ) -> NodeBuilder {
        self.builder = self.builder.with_secure_channels_registry(registry);
        self.clone()
    }

    /// Build top level services
    pub async fn build(&self, ctx: Context) -> Result<Node> {
        Ok(Node {
            context: ctx,
            secure_channels: self.builder.build(),
        })
    }
}
