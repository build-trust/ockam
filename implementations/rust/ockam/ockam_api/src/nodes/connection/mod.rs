mod plain_tcp;
mod project;
mod secure;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::flow_control::FlowControlPolicy::ProducerAllowMultiple;
use ockam_core::{async_trait, route, Address, CowStr, Route, LOCAL};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

pub(crate) use plain_tcp::PlainTcpInstantiator;
pub(crate) use project::ProjectInstantiator;
pub(crate) use secure::SecureChannelInstantiator;

pub struct Connection<'a> {
    pub ctx: &'a Context,
    pub addr: &'a MultiAddr,
    pub identity_name: Option<CowStr<'a>>,
    pub credential_name: Option<CowStr<'a>>,
    pub authorized_identities: Option<Vec<IdentityIdentifier>>,
    pub timeout: Option<Duration>,
    pub add_default_consumers: bool,
}

impl<'a> Connection<'a> {
    pub fn new(ctx: &'a Context, addr: &'a MultiAddr) -> Self {
        Self {
            ctx,
            addr,
            identity_name: None,
            credential_name: None,
            authorized_identities: None,
            timeout: None,
            add_default_consumers: false,
        }
    }

    #[allow(unused)]
    pub fn with_credential_name<T: Into<Option<CowStr<'a>>>>(mut self, credential_name: T) -> Self {
        self.credential_name = credential_name.into();
        self
    }

    pub fn with_authorized_identity<T: Into<Option<IdentityIdentifier>>>(
        mut self,
        authorized_identity: T,
    ) -> Self {
        self.authorized_identities = authorized_identity.into().map(|x| vec![x]);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn add_default_consumers(mut self) -> Self {
        self.add_default_consumers = true;
        self
    }
}

#[derive(Clone)]
pub struct ConnectionInstance {
    /// Transport route consists of only transport addresses,
    /// transport addresses are services which only carries over the payload without
    /// interpreting the content, and must be used to reach the other side of the connection.
    pub transport_route: Route,
    /// Resulting [`MultiAddr`] from the normalization, devoid of normalized protocols.
    /// A fully normalized [`MultiAddr`] contains only Service entries.
    pub normalized_addr: MultiAddr,
    /// The original provided [`MultiAddr`]
    pub original_addr: MultiAddr,
    /// A list of secure channel encryptors created for the connection.
    /// Needed to cleanup the connection resources when it must be closed.
    pub secure_channel_encryptors: Vec<Address>,
    /// A TCP worker address if used when instantiating the connection
    pub tcp_worker: Option<Address>,
    /// If a flow control was created
    pub flow_control_id: Option<FlowControlId>,
}

impl ConnectionInstance {
    /// Shorthand to add the address as consumer to the flow control
    pub fn add_consumer(&self, context: &Context, address: &Address) {
        if let Some(flow_control_id) = &self.flow_control_id {
            context.flow_controls().add_consumer(
                address.clone(),
                flow_control_id,
                ProducerAllowMultiple,
            );
        }
    }
}

impl Debug for ConnectionInstance {
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
pub struct ConnectionInstanceBuilder {
    original_multiaddr: MultiAddr,
    pub current_multiaddr: MultiAddr,
    pub transport_route: Route,
    pub flow_control_id: Option<FlowControlId>,
    pub secure_channel_encryptors: Vec<Address>,
    pub tcp_worker: Option<Address>,
}

impl Debug for ConnectionInstanceBuilder {
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

/// Represent changes to write to the [`ConnectionInstanceBuilder`]
pub struct Changes {
    /// If set, will overwrite the existing one on the [`ConnectionInstanceBuilder`] state
    pub flow_control_id: Option<FlowControlId>,
    /// Mandatory, will update the main [`MultiAddr`] in the [`ConnectionInstanceBuilder`]
    pub current_multiaddr: MultiAddr,
    /// Optional, to keep track of resources used add every time
    /// a new secure channel encryptor is created
    pub secure_channel_encryptors: Vec<Address>,
    /// Optional, to keep track of tcp worker when created for the connection
    pub tcp_worker: Option<Address>,
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
    ///                   see [`ConnectionInstanceBuilder::extract()`]
    ///                   and [`ConnectionInstanceBuilder::combine()`]
    ///
    /// The returned [`Changes`] will be used to update the builder state.
    async fn instantiate(
        &self,
        builder: &ConnectionInstanceBuilder,
        match_start: usize,
    ) -> Result<Changes, ockam_core::Error>;
}

impl ConnectionInstanceBuilder {
    pub fn new(multi_addr: MultiAddr) -> Self {
        ConnectionInstanceBuilder {
            transport_route: route![],
            original_multiaddr: multi_addr.clone(),
            current_multiaddr: multi_addr,
            secure_channel_encryptors: vec![],
            flow_control_id: None,
            tcp_worker: None,
        }
    }

    pub fn build(self) -> ConnectionInstance {
        ConnectionInstance {
            transport_route: self.transport_route,
            normalized_addr: self.current_multiaddr,
            original_addr: self.original_multiaddr,
            secure_channel_encryptors: self.secure_channel_encryptors,
            tcp_worker: self.tcp_worker,
            flow_control_id: self.flow_control_id,
        }
    }

    /// Used to instantiate a connection from a [`MultiAddr`]
    /// when called multiple times the instantiator order matters and it's up to the
    /// user make sure higher protocol abstraction are called before lower level ones
    pub async fn instantiate(
        mut self,
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
                    let mut changes = instantiator.instantiate(&self, start).await?;

                    self.current_multiaddr = changes.current_multiaddr;
                    self.secure_channel_encryptors
                        .append(&mut changes.secure_channel_encryptors);

                    if changes.tcp_worker.is_some() {
                        if self.tcp_worker.is_some() {
                            return Err(ockam_core::Error::new(
                                Origin::Transport,
                                Kind::Unsupported,
                                "multiple tcp connections created in a `MultiAddr`",
                            ));
                        }
                        self.tcp_worker = changes.tcp_worker;
                    }

                    if changes.flow_control_id.is_some() {
                        self.flow_control_id = changes.flow_control_id;
                    }
                    self.transport_route = self.recalculate_transport_route()?;
                }
                start += 1;
            }
        } else {
            self.transport_route = self.recalculate_transport_route()?;
        }

        Ok(Self {
            original_multiaddr: self.original_multiaddr,
            transport_route: self.transport_route,
            secure_channel_encryptors: self.secure_channel_encryptors,
            current_multiaddr: self.current_multiaddr,
            flow_control_id: self.flow_control_id,
            tcp_worker: self.tcp_worker,
        })
    }

    /// Calculate a 'transport route' from the [`MultiAddr`]
    fn recalculate_transport_route(&self) -> Result<Route, ockam_core::Error> {
        // assuming every plain service of the MultiAddr is transport except last
        let mut route = Route::new();
        let mut peekable = self.current_multiaddr.iter().peekable();
        while let Some(protocol) = peekable.next() {
            if protocol.code() == Service::CODE {
                if let Some(service) = protocol.cast::<Service>() {
                    let address = Address::new(LOCAL, &*service);
                    let is_last = peekable.peek().is_none();
                    // we usually want to skip the last entry since it's normally the destination
                    // but when a suffix route is appended (like in the inlet) is used
                    // the last piece could actually be a transport, in this case we allow
                    // last piece only if it's a just created secure channel
                    if is_last && !self.secure_channel_encryptors.contains(&address) {
                        break;
                    }
                    route = route.append(address);
                }
            }
        }

        Ok(route.into())
    }

    /// Extracts from a [`MultiAddr`] a piece, starting from `start` of length `length`.
    /// Returns the three pieces, (before, center, after).
    pub fn extract(
        multiaddr: &MultiAddr,
        start: usize,
        length: usize,
    ) -> (MultiAddr, MultiAddr, MultiAddr) {
        let (before, found_addr) = multiaddr.split(start);
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
