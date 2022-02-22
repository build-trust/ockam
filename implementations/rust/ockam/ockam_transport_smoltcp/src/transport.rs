use core::marker::PhantomData;
use core::str::FromStr;
use managed::ManagedSlice;
use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_core::{
    tcp::{
        router::{TcpRouter, TcpRouterHandle},
        traits::EndpointResolver,
    },
    TransportError,
};

use smoltcp::{
    iface::Routes,
    wire::{IpCidr, IpEndpoint},
};

use crate::{
    net::{Endpoints, InterfaceConfiguration, StackFacade},
    Clock, PortProvider,
};

/// High level management interface for TCP transports using smoltcp.
///
/// Mostly useful in `no-std` enviroments otherwise you might want to use the ockam-transport-tcp crate.
///
/// Be aware that only one `SmolTcpTransport` can exist per node, as it
/// registers itself as a router for the `TCP` address type.  Multiple
/// calls to [`SmolTcpTransport::create`](crate::SmolTcpTransport::create)
/// will fail.
///
/// To listen for incoming connections use
/// [`tcp.listen()`](crate::SmolTcpTransport::listen).
///
/// To register additional connections on an already initialised
/// `TcpTransport`, use [`tcp.connect()`](crate::SmolTcpTransport::connect).
/// This step is optional because the underlying TcpRouter is capable of lazily
/// establishing a connection upon arrival of an initial message.
///
/// # Examples
///
/// ## Explicitly connect to an endpoint.
///
/// ```rust
/// use ockam_transport_smoltcp::{SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
/// use ockam_transport_smoltcp::InterfaceConfiguration;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # use ockam_core::compat::sync::Mutex;
/// # use ockam_core::compat::collections::BTreeMap;
/// # use smoltcp::iface::Routes;
/// # use smoltcp::wire::{IpCidr, IpAddress, Ipv4Address};
/// # use std::str::FromStr;
/// # use lazy_static::lazy_static;
///
/// lazy_static! {
///     static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
/// }
///
/// # async fn test(ctx: Context) -> Result<()> {
/// let bind_ip_addr = "192.168.69.1:10222";
/// let default_gateway = "192.168.69.100";
/// let mut routes = Routes::new(BTreeMap::new());
/// routes
/// .add_default_ipv4_route(Ipv4Address::from_str(&default_gateway).unwrap())
/// .unwrap();
/// let mut configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
///     [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
///     [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
///     &*DEVICE,
/// );
/// configuration.set_routes(routes);
///
/// let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock)).await?;
///
/// tcp.connect("192.168.69.100:10222").await?;
/// # Ok(()) }
/// ```
///
/// ## The same `TcpTransport` can also bind to multiple ports.
///
/// ```rust
/// use ockam_transport_smoltcp::{SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
/// use ockam_transport_smoltcp::InterfaceConfiguration;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # use ockam_core::compat::sync::Mutex;
/// # use ockam_core::compat::collections::BTreeMap;
/// # use smoltcp::iface::Routes;
/// # use smoltcp::wire::{IpCidr, IpAddress, Ipv4Address};
/// # use std::str::FromStr;
/// # use lazy_static::lazy_static;
///
/// lazy_static! {
///     static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
/// }
///
/// # async fn test(ctx: Context) -> Result<()> {
///
/// let bind_ip_addr = "192.168.69.1:10222";
/// let mut configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
///     [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
///     [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
///     &*DEVICE,
/// );
///
/// let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock)).await?;
///
/// tcp.listen(10222).await?;
/// tcp.listen(10333).await?;
/// # Ok(()) }
/// ```
pub struct SmolTcpTransport<P> {
    router_handle: TcpRouterHandle<SmolTcpEndpointResolver<P>>,
    stack: StackFacade,
}

impl<P> SmolTcpTransport<P>
where
    P: PortProvider + Send + Sync + 'static,
{
    /// Create a new TCP transport and router for the current node
    ///
    /// If you don't provide a [Clock] you will need to manually [poll](StackFacade::poll) the stack. However, if the `Clock` is provided you don't need to poll the stack(you can still do it but there's no gain in doing it).
    ///
    /// To get the stack to poll use [SmolTcpTransport::get_stack].
    /// ```rust
    /// use ockam_transport_smoltcp::{SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
    /// use ockam_transport_smoltcp::InterfaceConfiguration;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # use ockam_core::compat::sync::Mutex;
    /// # use smoltcp::iface::Routes;
    /// # use smoltcp::wire::IpCidr;
    /// # use smoltcp::wire::IpAddress;
    /// # use std::str::FromStr;
    /// # use lazy_static::lazy_static;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// lazy_static! {
    ///     static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
    /// }
    ///
    /// let bind_ip_addr = "192.168.69.1:10222";
    /// let configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
    ///     [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
    ///     [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
    ///     &*DEVICE,
    /// );
    ///
    /// let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock)).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create<C, T, U>(
        ctx: &Context,
        config: InterfaceConfiguration<T, U>,
        clock: Option<C>,
    ) -> Result<Self>
    where
        C: Clock + Send + Sync + 'static,
        T: Into<ManagedSlice<'static, IpCidr>>,
        U: Into<Routes<'static>>,
        P: PortProvider + Send + 'static,
    {
        let stack = StackFacade::init_stack(config);
        if let Some(clock) = clock {
            stack.run(clock).await;
        }
        let router_handle = TcpRouter::<_, _, SmolTcpEndpointResolver<P>>::register(
            ctx,
            stack,
            crate::CLUSTER_NAME,
        )
        .await?;

        Ok(Self {
            router_handle,
            stack,
        })
    }

    /// Start listening to incoming connections on an existing transport
    ///
    /// ## Parameeters
    /// - `bind_port`: the port the interface will be listening at.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ockam_transport_smoltcp::{SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
    /// use ockam_transport_smoltcp::InterfaceConfiguration;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # use ockam_core::compat::sync::Mutex;
    /// # use ockam_core::compat::collections::BTreeMap;
    /// # use smoltcp::iface::Routes;
    /// # use smoltcp::wire::{IpCidr, IpAddress, Ipv4Address};
    /// # use std::str::FromStr;
    /// # use lazy_static::lazy_static;
    ///
    /// lazy_static! {
    ///   static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
    /// }
    ///
    /// # async fn test(ctx: Context) -> Result<()> {
    ///
    /// let bind_ip_addr = "192.168.69.1:10222";
    /// let mut configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
    ///     [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
    ///     [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
    ///     &*DEVICE,
    /// );
    ///
    /// let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock)).await?;
    ///
    /// tcp.listen(10222).await?;
    /// # Ok(()) }
    /// ```
    pub async fn listen(&self, bind_port: u16) -> Result<()> {
        self.router_handle.bind(bind_port, self.stack).await?;
        Ok(())
    }

    /// Manually establish an outgoing TCP connection on an existing transport.
    /// This step is optional because the underlying TcpRouter is capable of lazily establishing
    /// a connection upon arrival of the initial message.
    ///
    /// ```rust
    /// use ockam_transport_smoltcp::{SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
    /// use ockam_transport_smoltcp::InterfaceConfiguration;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # use ockam_core::compat::sync::Mutex;
    /// # use ockam_core::compat::collections::BTreeMap;
    /// # use smoltcp::iface::Routes;
    /// # use smoltcp::wire::{IpCidr, IpAddress, Ipv4Address};
    /// # use std::str::FromStr;
    /// # use lazy_static::lazy_static;
    ///
    /// lazy_static! {
    ///   static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
    /// }
    ///
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let bind_ip_addr = "192.168.69.1:10222";
    /// let default_gateway = "192.168.69.100";
    /// let mut routes = Routes::new(BTreeMap::new());
    /// routes
    /// .add_default_ipv4_route(Ipv4Address::from_str(&default_gateway).unwrap())
    /// .unwrap();
    /// let mut configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
    /// [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
    /// [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
    /// &*DEVICE,
    /// );
    /// configuration.set_routes(routes);
    ///
    /// let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock)).await?;
    ///
    /// tcp.connect("192.168.69.100:10222").await?;
    /// # Ok(()) }
    /// ```
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        self.router_handle.connect(peer, self.stack).await
    }

    /// Returns the `StackFacade` corresponding to the smoltcp stack.
    ///
    /// This is only useful if you will [poll](StackFacade::poll) it manually instead of depending on the polling mechanism in this crate.
    ///
    /// If you want to poll it manually make sure to *not* provide a `Clock` to [SmolTcpTransport::create]
    pub fn get_stack(&self) -> StackFacade {
        self.stack
    }
}

struct SmolTcpEndpointResolver<T>(PhantomData<T>);

impl<T> EndpointResolver for SmolTcpEndpointResolver<T>
where
    T: PortProvider,
{
    type Hostnames = &'static [&'static str];

    type Peer = Endpoints<IpEndpoint, IpEndpoint>;

    fn resolve_endpoint(peer: &str) -> Result<(Self::Peer, Self::Hostnames), ockam_core::Error> {
        let empty: &'static [&'static str] = &[];
        Ok((
            Endpoints {
                remote: IpEndpoint::from_str(peer).map_err(|_| TransportError::InvalidAddress)?,
                local: T::next_port().into(),
            },
            empty,
        ))
    }
}
