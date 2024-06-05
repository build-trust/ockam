mod plain_tcp;
mod project;
mod secure;

use ockam::tcp::TcpConnection;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::Result;
use ockam_core::{async_trait, route, Address, Route, LOCAL};
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::NodeManager;
pub(crate) use plain_tcp::PlainTcpInstantiator;
pub(crate) use project::ProjectInstantiator;
pub(crate) use secure::SecureChannelInstantiator;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[derive(Clone)]
pub struct Connection {
    /// Transport route consists of only transport addresses,
    /// transport addresses are services which only carries over the payload without
    /// interpreting the content, and must be used to reach the other side of the connection.
    transport_route: Route,
    /// Resulting [`MultiAddr`] from the normalization, devoid of normalized protocols.
    /// A fully normalized [`MultiAddr`] contains only Service entries.
    pub(crate) normalized_addr: MultiAddr,
    /// The original provided [`MultiAddr`]
    original_addr: MultiAddr,
    /// A list of secure channel encryptors created for the connection.
    /// Needed to cleanup the connection resources when it must be closed.
    pub(crate) secure_channel_encryptors: Vec<Address>,
    /// A TCP worker address if used when instantiating the connection
    pub(crate) tcp_connection: Option<TcpConnection>,
    /// If a flow control was created
    flow_control_id: Option<FlowControlId>,
}

impl Connection {
    /// Shorthand to add the address as consumer to the flow control
    pub fn add_consumer(&self, context: Arc<Context>, address: &Address) {
        if let Some(flow_control_id) = &self.flow_control_id {
            context
                .flow_controls()
                .add_consumer(address.clone(), flow_control_id);
        }
    }

    pub fn add_default_consumers(&self, ctx: Arc<Context>) {
        self.add_consumer(ctx.clone(), &DefaultAddress::KEY_EXCHANGER_LISTENER.into());
        self.add_consumer(ctx.clone(), &DefaultAddress::SECURE_CHANNEL_LISTENER.into());
        self.add_consumer(ctx.clone(), &DefaultAddress::UPPERCASE_SERVICE.into());
        self.add_consumer(ctx, &DefaultAddress::ECHO_SERVICE.into());
    }

    pub fn transport_route(&self) -> Route {
        self.transport_route.clone()
    }

    pub fn route(&self) -> Result<Route> {
        local_multiaddr_to_route(&self.normalized_addr).map_err(|_| {
            ApiError::core(format!(
                "Couldn't convert MultiAddr to route: normalized_addr={}",
                self.normalized_addr
            ))
        })
    }

    pub async fn close(&self, context: &Context, node_manager: &NodeManager) -> Result<()> {
        for encryptor in &self.secure_channel_encryptors {
            if let Err(error) = node_manager.delete_secure_channel(context, encryptor).await {
                match error.code().kind {
                    Kind::NotFound => {
                        debug!("cannot find and delete secure channel `{encryptor}`: {error}");
                    }
                    _ => Err(ockam_core::Error::new(
                        Origin::Node,
                        Kind::Internal,
                        format!(
                            "Failed to delete secure channnel with address {address}. {error}",
                            address = encryptor,
                        ),
                    ))?,
                }
            }
        }

        if let Some(tcp_connection) = self.tcp_connection.as_ref() {
            let address = tcp_connection.sender_address().clone();
            if let Err(error) = node_manager.tcp_transport.disconnect(address.clone()).await {
                match error.code().kind {
                    Kind::NotFound => {
                        debug!("cannot find and disconnect tcp worker `{tcp_connection}`");
                    }
                    _ => Err(ockam_core::Error::new(
                        Origin::Node,
                        Kind::Internal,
                        format!("Failed to remove inlet with alias {address}. {}", error),
                    ))?,
                }
            }
        }

        Ok(())
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        write!(f, " transport_route: {},", self.transport_route)?;
        write!(f, " normalized_addr: {},", self.normalized_addr)?;
        write!(f, " original_addr: {},", self.original_addr)?;
        write!(f, " flow_control_id: {:?},", self.flow_control_id.as_ref())?;
        write!(
            f,
            " secure_channel_encryptors: {:?} ",
            self.secure_channel_encryptors
        )?;
        write!(f, "}}")
    }
}

/// Used to instantiate a connection from a [`MultiAddr`]
#[derive(Clone)]
pub(crate) struct ConnectionBuilder {
    original_multiaddr: MultiAddr,
    pub(crate) current_multiaddr: MultiAddr,
    pub(crate) transport_route: Route,
    pub(crate) flow_control_id: Option<FlowControlId>,
    pub(crate) secure_channel_encryptors: Vec<Address>,
    pub(crate) tcp_connection: Option<TcpConnection>,
}

impl Debug for ConnectionBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        write!(f, " original_multiaddr: {},", self.original_multiaddr)?;
        write!(f, " current_multiaddr: {},", self.current_multiaddr)?;
        write!(f, " neutral_route: {},", self.transport_route)?;
        write!(f, " flow_control_id: {:?},", self.flow_control_id.as_ref())?;
        write!(
            f,
            " secure_channel_encryptors: {:?} ",
            self.secure_channel_encryptors
        )?;
        write!(f, "}}")
    }
}

/// Represent changes to write to the [`ConnectionBuilder`]
pub struct Changes {
    /// If set, will overwrite the existing one on the [`ConnectionBuilder`] state
    pub flow_control_id: Option<FlowControlId>,
    /// Mandatory, will update the main [`MultiAddr`] in the [`ConnectionBuilder`]
    pub current_multiaddr: MultiAddr,
    /// Optional, to keep track of resources used add every time
    /// a new secure channel encryptor is created
    pub secure_channel_encryptors: Vec<Address>,
    /// Optional, to keep track of tcp worker when created for the connection
    pub tcp_connection: Option<TcpConnection>,
}

/// Takes in a [`MultiAddr`] and instantiate it, can be implemented for any protocol.
/// Each [`Instantiator`] is limited to a single [`Match`] list.
#[async_trait]
pub trait Instantiator: Send + Sync + 'static {
    /// Returns a list of matches for the search within the [`MultiAddr`]
    fn matches(&self) -> Vec<Match>;

    /// Instantiate the match found within the [`MultiAddr`] using [`Instantiator::matches()`]
    /// * `builder` - Current state of the builder, read-only
    /// * `match_start` - The start of the match within the [`MultiAddr`],
    ///                   see [`ConnectionBuilder::extract()`]
    ///                   and [`ConnectionBuilder::combine()`]
    ///
    /// The returned [`Changes`] will be used to update the builder state.
    async fn instantiate(
        &self,
        ctx: Arc<Context>,
        node_manager: &NodeManager,
        transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, ockam_core::Error>;
}

impl ConnectionBuilder {
    pub fn new(multi_addr: MultiAddr) -> Self {
        ConnectionBuilder {
            transport_route: route![],
            original_multiaddr: multi_addr.clone(),
            current_multiaddr: multi_addr,
            secure_channel_encryptors: vec![],
            flow_control_id: None,
            tcp_connection: None,
        }
    }

    pub fn build(self) -> Connection {
        Connection {
            transport_route: self.transport_route,
            normalized_addr: self.current_multiaddr,
            original_addr: self.original_multiaddr,
            secure_channel_encryptors: self.secure_channel_encryptors,
            tcp_connection: self.tcp_connection,
            flow_control_id: self.flow_control_id,
        }
    }

    /// Used to instantiate a connection from a [`MultiAddr`]
    /// when called multiple times the instantiator order matters and it's up to the
    /// user make sure higher protocol abstraction are called before lower level ones
    pub async fn instantiate(
        mut self,
        ctx: Arc<Context>,
        node_manager: &NodeManager,
        instantiator: impl Instantiator,
    ) -> Result<Self, ockam_core::Error> {
        //executing a regex-like search, shifting the starting point one by one
        //not efficient by any mean, but it shouldn't be an issue
        let codes = instantiator.matches();
        let length = codes.len();
        let mut start = 0;

        if self.current_multiaddr.len() > length {
            while start < self.current_multiaddr.len() - length {
                if self.current_multiaddr.matches(start, &codes) {
                    // the transport route should include only the pieces before the match
                    self.transport_route = self
                        .recalculate_transport_route(
                            &ctx,
                            self.current_multiaddr.split(start).0,
                            false,
                        )
                        .await?;
                    let mut changes = instantiator
                        .instantiate(
                            ctx.clone(),
                            node_manager,
                            self.transport_route.clone(),
                            self.extract(start, instantiator.matches().len()),
                        )
                        .await?;

                    self.current_multiaddr = changes.current_multiaddr;
                    self.secure_channel_encryptors
                        .append(&mut changes.secure_channel_encryptors);

                    if changes.tcp_connection.is_some() {
                        if self.tcp_connection.is_some() {
                            return Err(ockam_core::Error::new(
                                Origin::Transport,
                                Kind::Unsupported,
                                "multiple tcp connections created in a `MultiAddr`",
                            ));
                        }
                        self.tcp_connection = changes.tcp_connection;
                    }

                    if changes.flow_control_id.is_some() {
                        self.flow_control_id = changes.flow_control_id;
                    }
                }
                start += 1;
            }
        }

        self.transport_route = self
            .recalculate_transport_route(&ctx, self.current_multiaddr.clone(), true)
            .await?;

        Ok(Self {
            original_multiaddr: self.original_multiaddr,
            transport_route: self.transport_route,
            secure_channel_encryptors: self.secure_channel_encryptors,
            current_multiaddr: self.current_multiaddr,
            flow_control_id: self.flow_control_id,
            tcp_connection: self.tcp_connection,
        })
    }

    /// Calculate a 'transport route' from the [`MultiAddr`]
    async fn recalculate_transport_route(
        &self,
        ctx: &Context,
        current_before: MultiAddr,
        last_pass: bool,
    ) -> Result<Route, ockam_core::Error> {
        // only when performing the last pass we assume every plain service of the MultiAddr
        // is transport except last
        let mut route = Route::new();
        let mut peekable = current_before.iter().peekable();
        while let Some(protocol) = peekable.next() {
            if protocol.code() == Service::CODE {
                if let Some(service) = protocol.cast::<Service>() {
                    let address = Address::new(LOCAL, &*service);
                    let is_last = peekable.peek().is_none();

                    // we usually want to skip the last entry since it's normally the destination
                    // but when a suffix route is appended (like in the inlet) is used
                    // the last piece could actually be a transport, in this case we allow
                    // last piece only if it's a terminal (a service connecting to another node)
                    if last_pass && is_last {
                        let is_terminal = ctx
                            .read_metadata(address.clone())
                            .await
                            .ok()
                            .flatten()
                            .map(|m| m.is_terminal)
                            .unwrap_or(false);

                        if !is_terminal {
                            break;
                        }
                    }
                    route = route.append(address);
                }
            }
        }

        Ok(route.into())
    }

    /// Extracts from a [`MultiAddr`] a piece, starting from `start` of length `length`.
    /// Returns the three pieces, (before, center, after).
    fn extract(&self, start: usize, length: usize) -> (MultiAddr, MultiAddr, MultiAddr) {
        let (before, found_addr) = self.current_multiaddr.split(start);
        let (part_to_replace, after) = found_addr.split(length);
        (before, part_to_replace, after)
    }

    /// Combine together three [`MultiAddr`], one after the other, in order.
    pub fn combine(
        before: MultiAddr,
        replaced: MultiAddr,
        after: MultiAddr,
    ) -> Result<MultiAddr, ockam_core::Error> {
        let mut new_multiaddr = MultiAddr::new(before.registry().clone());

        new_multiaddr.try_extend(before.iter())?;
        new_multiaddr.try_extend(replaced.iter())?;
        new_multiaddr.try_extend(after.iter())?;

        Ok(new_multiaddr)
    }
}
