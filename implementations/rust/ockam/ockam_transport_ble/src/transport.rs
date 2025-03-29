use core::str::FromStr;
use core::sync::atomic::{AtomicBool, Ordering};

use ockam_core::Result;
use ockam_node::Context;

use crate::driver::{BleClient, BleServer};
use crate::driver::{BleClientDriver, BleServerDriver, BleStreamDriver};
use crate::router::{BleRouter, BleRouterHandle};
use crate::BleAddr;

/// High level management interface for BLE transports
///
/// Be aware that only one `BleTransport` can exist per node, as it
/// registers itself as a router for the `BLE` address type.  Multiple
/// calls to [`BleTransport::create`](crate::BleTransport::create)
/// will panic.
///
/// To register additional connections on an already initialised
/// `BleTransport`, use
/// [`ble.connect()`](crate::BleTransport::connect).  To listen for
/// incoming connections use
/// [`ble.listen()`](crate::BleTransport::listen)
///
/// ```rust
/// use ockam_transport_ble::{BleClient, BleTransport};
/// use ockam_transport_ble::driver::btleplug::BleAdapter;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
///     // Create a ble_client
///     let ble_adapter = BleAdapter::try_new().await?;
///     let ble_client = BleClient::with_adapter(ble_adapter);
///
///     // Initialize the BLE Transport.
///     let ble = BleTransport::create(&ctx)?;
///
///     // Try to connect to BleServer
///     ble.connect(ble_client, "ockam_ble_1".to_string()).await?;
/// # Ok(()) }
/// ```
pub struct BleTransport {
    router_handle: BleRouterHandle,
}

impl BleTransport {
    /// Create a new BLE transport and router for the current node
    pub fn create(ctx: &Context) -> Result<Self> {
        static CREATED: AtomicBool = AtomicBool::new(false);
        if CREATED.swap(true, Ordering::SeqCst) {
            panic!("You may only create one BleTransport per node.");
        }

        let router_handle = BleRouter::register(ctx)?;

        Ok(Self { router_handle })
    }

    /// Establish an outgoing BLE connection on an existing transport
    ///
    /// Starts a new pair of Ble connection workers
    ///
    /// One worker handles outgoing messages, while another handles
    /// incoming messages. The local worker address is chosen based on
    /// the peer the worker is meant to be connected to.
    pub async fn connect<
        A: BleClientDriver + BleStreamDriver + Send + 'static,
        S: AsRef<str> + core::fmt::Debug,
    >(
        &self,
        ble_client: BleClient<A>,
        peer: S,
    ) -> Result<()> {
        self.router_handle.connect(ble_client, peer.as_ref()).await
    }

    /// Start listening to incoming connections on an existing transport
    pub async fn listen<
        A: BleServerDriver + BleStreamDriver + Send + 'static,
        S: AsRef<str> + core::fmt::Debug,
    >(
        &self,
        ble_server: BleServer<A>,
        listen_addr: S,
    ) -> Result<()> {
        let bind_addr = BleAddr::from_str(listen_addr.as_ref())?;
        debug!("BleTransport::listen -> {:?}", listen_addr);
        self.router_handle.bind(ble_server, bind_addr).await?;

        Ok(())
    }
}
