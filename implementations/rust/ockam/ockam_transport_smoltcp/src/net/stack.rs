use core::fmt::Display;
use core::future::Future;
use futures::pin_mut;
use lazy_static::lazy_static;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Mutex;
use ockam_core::compat::task::{Context, Poll};
use ockam_transport_core::tcp::traits::{TcpAccepter, TcpBinder, TcpStreamConnector};
use ockam_transport_core::TransportError;
use smoltcp::socket::{Socket, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, IpEndpoint};
use smoltcp::Result;
use smoltcp::{
    iface::{
        Context as SmolContext, Interface, InterfaceBuilder, NeighborCache, Routes, SocketHandle,
    },
    socket::TcpState,
};
use tracing::{error, trace};

use super::{
    device::{Device, DeviceAdapter},
    timer::Clock,
};
use super::{AsyncTcpSocket, AsyncTcpStream};

// Tap devices only make sense in std
#[cfg(feature = "std")]
pub use super::TunTapDevice;

lazy_static! {
    // The global stack that will be used by this module
    static ref STACK: Mutex<Option<Stack>> = Mutex::new(None);
}

use managed::{ManagedMap, ManagedSlice};

/// Configuration for the smoltcp stack.
///
/// All the configuration will be consumed by the stack `init` and you will not be able to change it afterwards.
#[derive(Clone)]
pub struct InterfaceConfiguration<T, U>
where
    T: Into<ManagedSlice<'static, IpCidr>>,
    U: Into<Routes<'static>>,
{
    eth_addr: [u8; 6],
    ip_addrs: T,
    routes: Option<U>,
    device: &'static Mutex<dyn Device + Send>,
}

impl<T, U> InterfaceConfiguration<T, U>
where
    T: Into<ManagedSlice<'static, IpCidr>>,
    U: Into<Routes<'static>>,
{
    // We don't support DHCP right now so we need to set an IpAddr
    /// Create a new configuration for the smoltcp stack.
    ///
    /// ## Parameters:
    /// - `eth_addr`: the 6 octets of the ethernet addres.
    /// - `ip`: Ip address that will be used by the interface.
    /// - `device`: A device that implements [Device], this will be polled for packages and it will wake our waker when they become available.
    pub fn new(eth_addr: [u8; 6], ip: T, device: &'static Mutex<dyn Device + Send>) -> Self {
        Self {
            eth_addr,
            ip_addrs: ip,
            routes: None,
            device,
        }
    }

    /// Sets the ip routes.
    pub fn set_routes(&mut self, routes: U) {
        self.routes = Some(routes);
    }

    fn get_routes(&mut self) -> Routes<'static> {
        match self.routes.take() {
            Some(routes) => routes.into(),
            None => Routes::new(ManagedMap::Borrowed(&mut [])),
        }
    }
}

fn create_iface<T, U>(mut config: InterfaceConfiguration<T, U>) -> Interface<'static, DeviceAdapter>
where
    T: Into<ManagedSlice<'static, IpCidr>>,
    U: Into<Routes<'static>>,
{
    let eth_addr = EthernetAddress(config.eth_addr);
    let device = DeviceAdapter::new(config.device);

    // TODO allocation
    // These allocation should be easy to remove by asking the user to provide these resources
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    // TODO: If we don't alloc here and use statically allocated storage remember to update get_tcp_socket so we don't panic
    let socket_storage = vec![];

    let iface = InterfaceBuilder::new(device, socket_storage)
        .hardware_addr(eth_addr.into())
        .neighbor_cache(neighbor_cache)
        .routes(config.get_routes())
        .ip_addrs(config.ip_addrs)
        .finalize();

    iface
}

struct Stack {
    iface: Interface<'static, DeviceAdapter>,
}

/// Facade into the underlying stack.
///
/// This facade provides type safety for the stack making sure that it has been initialized before using it.
///
/// This is used internally but if you want to [poll](StackFacade::poll) the stack manually instead of depending on it running on the background you need to use this struct.
#[derive(Clone, Debug, Copy)]
pub struct StackFacade(());

#[derive(Clone, Debug)]
pub(crate) struct Endpoints<T, U> {
    pub remote: T,
    pub local: U,
}

impl<T, U> Display for Endpoints<T, U>
where
    T: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // For routing purposes we only care for the remote endpoint.
        self.remote.fmt(f)
    }
}

#[async_trait]
impl<T, U> TcpStreamConnector<Endpoints<T, U>> for StackFacade
where
    T: Into<IpEndpoint> + Send + 'static,
    U: Into<IpEndpoint> + Send + 'static,
{
    type Stream = AsyncTcpStream;

    async fn connect(
        &self,
        Endpoints { remote, local }: Endpoints<T, U>,
    ) -> core::result::Result<Self::Stream, TransportError> {
        let mut socket = self.get_async_tcp_socket();
        AsyncTcpSocket::connect(&mut socket, remote, local)
            .await
            .map_err(TransportError::from)?;

        Ok(socket.into_stream())
    }
}

impl StackFacade {
    pub(crate) fn init_stack<T, U>(config: InterfaceConfiguration<T, U>) -> Self
    where
        T: Into<ManagedSlice<'static, IpCidr>>,
        U: Into<Routes<'static>>,
    {
        if STACK.lock().unwrap().is_none() {
            Stack::init(config);
        } else {
            panic!("The stack should never be initialized twice");
        }

        Self(())
    }

    pub(crate) fn get_async_tcp_socket(&self) -> AsyncTcpSocket {
        Self::with_stack(|stack| stack.get_async_tcp_socket(*self))
    }

    pub(crate) async fn accept<T: Into<IpEndpoint>>(
        &self,
        binding_endpoint: T,
    ) -> Result<(AsyncTcpStream, IpEndpoint)> {
        let mut new_socket = Self::with_stack(|stack| stack.get_async_tcp_socket(*self));
        new_socket.listen(binding_endpoint).await?;
        Ok(new_socket.accept().await)
    }

    pub(crate) fn with_handle<R>(
        &self,
        handle: SocketHandle,
        f: impl FnOnce(&mut TcpSocket, &mut SmolContext) -> R,
    ) -> R {
        Self::with_stack(|stack| stack.with(handle, f))
    }

    /// Polls the underlying stack.
    ///
    /// It uses the `context` to extract a `waker` that will be signaled (either by the [Device], by a timeout or by any action taken to a socket).
    ///
    /// Note that you will never have to use this function or concern yourself with the [StackFacade] if you pass a [Clock] to [SmolTcpTransport::create](crate::SmolTcpTransport::create)
    pub fn poll(&self, cx: &mut Context, timestamp: impl Into<Instant>) {
        Self::with_stack(|stack| stack.poll_iface(cx, timestamp));
    }

    pub(crate) async fn run<C>(&self, clock: C)
    where
        C: Clock + Send + Sync + 'static,
    {
        ockam_node::spawn(async move {
            futures::future::poll_fn::<(), _>(|cx| {
                let timestamp = clock.now();
                Self::with_stack(|stack| {
                    stack.poll_iface(cx, timestamp);
                });
                Poll::Pending
            })
            .await;
        });
    }

    fn with_stack<R>(f: impl FnOnce(&mut Stack) -> R) -> R {
        let mut stack = STACK.lock().unwrap();
        let stack = stack
            .as_mut()
            .expect("There should be no way to access the stack without inititializing it first");
        f(stack)
    }
}

pub struct StackTcpAccepter {
    stack: StackFacade,
    local_endpoint: IpEndpoint,
}

#[async_trait]
impl<A> TcpBinder<A> for StackFacade
where
    A: Into<IpEndpoint> + Send + 'static,
{
    type Listener = StackTcpAccepter;
    async fn bind(&self, addr: A) -> core::result::Result<Self::Listener, TransportError> {
        Ok(StackTcpAccepter {
            stack: *self,
            local_endpoint: addr.into(),
        })
    }
}

#[async_trait]
impl TcpAccepter for StackTcpAccepter {
    type Stream = AsyncTcpStream;
    type Peer = IpEndpoint;
    async fn accept(&mut self) -> core::result::Result<(Self::Stream, Self::Peer), TransportError> {
        self.stack
            .accept(self.local_endpoint)
            .await
            .map_err(TransportError::from)
    }
}

impl Stack {
    fn init<T, U>(config: InterfaceConfiguration<T, U>)
    where
        T: Into<ManagedSlice<'static, IpCidr>>,
        U: Into<Routes<'static>>,
    {
        let iface = create_iface(config);
        let mut stack = STACK.lock().unwrap();
        *stack = Some(Self { iface });
    }

    fn with<R>(
        &mut self,
        handle: SocketHandle,
        f: impl FnOnce(&mut TcpSocket, &mut SmolContext) -> R,
    ) -> R {
        let (socket, cx) = self.iface.get_socket_and_context(handle);
        let res = f(socket, cx);
        if let Some(ref waker) = self.iface.device().get_waker() {
            waker.wake_by_ref();
        }

        res
    }

    fn get_tcp_socket(&mut self) -> SocketHandle {
        // Reuse closed sockets if possible.
        // Note: adding a socket already iterates through all the socket handles so this is probably not a problem
        let closed_socket = self.iface.sockets().find_map(|(socket_handle, socket)| {
            // Note: This is only a irrefutable pattern when the only kind of socket enabled
            // is tcp. Which might not be the case in a downstream crate.
            #[allow(irrefutable_let_patterns)]
            if let Socket::Tcp(socket) = socket {
                match socket.state() {
                    TcpState::Closed => Some(socket_handle),
                    _ => None,
                }
            } else {
                None
            }
        });

        if let Some(socket_handle) = closed_socket {
            socket_handle
        } else {
            // TODO allocation
            // Normally to prevent this allocation we would have the user pass on the buffers when creating the socket but in our case
            // We need to support the `accept` method (look at the sibling tcp module) that automatically creates gets a socket after a new connection.
            // So we need to have some way to allocate new buffers for each new incoming connection(sans reused sockets). We could use a pool(Like we've already done for the device's tokens)
            // but maybe we could create a trait for `get_new_socket_buffer` and use that so that the user can chose whatever allocation method.
            let socket_rx_buff = TcpSocketBuffer::new(vec![0; 65535]);
            let socket_tx_buff = TcpSocketBuffer::new(vec![0; 65535]);

            let socket = TcpSocket::new(socket_rx_buff, socket_tx_buff);
            self.iface.add_socket(socket) // TODO! This can panic if we are using statically allocated socket storage, remember to change this when we allow for that!!
        }
    }

    fn get_async_tcp_socket(&mut self, stack_facade: StackFacade) -> AsyncTcpSocket {
        let socket_handle = self.get_tcp_socket();
        AsyncTcpSocket::new(socket_handle, stack_facade)
    }

    fn poll_iface(&mut self, cx: &mut Context, timestamp: impl Into<Instant>) {
        trace!("Polling interface");
        // Register the waker for the interface (Used to wake each time a change ocurrs in the interface)
        self.iface.device_mut().register_waker(cx.waker());

        // Poll the interface
        let timestamp = timestamp.into();
        match self.iface.poll(timestamp) {
            Ok(_) => {}
            Err(e) => {
                error!("poll error: {}", e);
                cx.waker().wake_by_ref();
            }
        }

        // Get recomended delay and register waker for delay
        let delay = self.iface.poll_delay(timestamp);

        if let Some(delay) = delay {
            let time = ockam_node::tokio::time::sleep(
                ockam_node::tokio::time::Duration::from_millis(delay.millis()),
            );
            pin_mut!(time);
            if time.poll(cx).is_ready() {
                cx.waker().wake_by_ref();
            }
        }
    }
}
