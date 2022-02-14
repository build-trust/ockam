//! Bluetooth platform drivers

#![allow(unused_imports, unused_mut, unused_variables)]

/// support for ST Micro BlueNRG Bluetooth radios
#[cfg(feature = "use_bluetooth_hci")]
pub mod bluetooth_hci;

/// support for OS bluetooth
#[cfg(all(
    feature = "use_btleplug",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
pub mod btleplug;

#[cfg(not(feature = "std"))]
mod mutex;
mod packet;
mod stream;

pub(crate) use packet::PacketBuffer;
pub(crate) use stream::{AsyncStream, Sink, Source};

use crate::BleAddr;
use ockam_core::{async_trait, compat::boxed::Box};

/// The minimum MTU required by the BLE spec. Many devices
/// (e.g. bluenrg_ms) don't allow for configuration higher than this.
pub const MTU: usize = 23;

/// Maximum length of characteristic values (250 is the longest
/// allowed by the BLE specification)
///
/// Hard-limited for now according to MTU:
///
/// MTU - 5 (packet fields) = 18 bytes of payload
pub const CHARACTERISTIC_VALUE_LENGTH: usize = MTU - 5;

/// Maximum length of ockam messages
pub const MAX_OCKAM_MESSAGE_LENGTH: usize = 1024;

/// Ockam BLE service and endpoints are encoded as UUID v5 identifiers
/// within the ns:OID namespace:
///
///     iso.identified-organization.ockam.transport.ble (1.3.214.9.4)
///
/// Ockam OID information can be obtained at:
///
///     http://oid-info.com/get/1.3.214.9.4
pub mod uuid {
    /// BLE Service UUID (rfc1700 / big-endian)
    /// Namespace: ns:OID
    /// Name: iso.identified-organization.ockam.transport.ble.service (1.3.214.9.4.1)
    pub const SERVICE: u128 = 0x424c001f_48f6_5fff_bc3e_84e00c335dbb;

    /// BLE Read UUID (rfc1700 / big-endian)
    /// Namespace: ns:OID
    /// Name: iso.identified-organization.ockam.transport.ble.read (1.3.214.9.4.2)
    pub const READ: u128 = 0xbc911b36_320c_5d3f_90ed_842a6e57cefb;

    /// Default BLE Write UUID (rfc1700 / big-endian)
    /// Namespace: ns:OID
    /// Name: iso.identified-organization.ockam.transport.ble.write (1.3.214.9.4.3)
    pub const WRITE: u128 = 0x18c90ff2_8d94_5f20_8084_c16ae3f6a9a2;
}

/// BleEvent
#[derive(Debug)]
pub enum BleEvent<'a> {
    None,
    Unknown,
    ConnectionComplete,
    Received(&'a [u8]),
    DisconnectionComplete,
}

/// Implement the BleClientDriver trait if you want to allow your
/// hardware to function as a BLE Client
#[async_trait]
pub trait BleClientDriver {
    async fn scan(&mut self, ble_addr: &BleAddr) -> ockam::Result<()>;
    async fn connect(&mut self) -> ockam::Result<()>;
}

/// Implement the BleServerDriver trait if you want to allow your
/// hardware to function as a BLE Client
#[async_trait]
pub trait BleServerDriver {
    async fn bind(&mut self, ble_addr: &BleAddr) -> ockam::Result<()>;
    async fn start_advertising(&mut self) -> ockam::Result<()>;
}

/// Implement the BleStreamDriver to transmit and receive data from
/// your hardware
#[async_trait]
pub trait BleStreamDriver {
    async fn poll<'b>(&mut self, buffer: &'b mut [u8]) -> ockam::Result<BleEvent<'b>>;
    async fn write(&mut self, buffer: &[u8]) -> ockam::Result<()>;
}

/// A BLE client that initiates GATT commands and requests, and
/// accepts responses from a BLE server.
pub struct BleClient<A>
where
    A: BleClientDriver + BleStreamDriver + Send,
{
    inner: A,
}

impl<A> BleClient<A>
where
    A: BleClientDriver + BleStreamDriver + Send,
{
    pub fn with_adapter(inner: A) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<A> BleClientDriver for BleClient<A>
where
    A: BleClientDriver + BleStreamDriver + Send,
{
    async fn scan(&mut self, ble_addr: &BleAddr) -> ockam::Result<()> {
        self.inner.scan(ble_addr).await
    }

    async fn connect(&mut self) -> ockam::Result<()> {
        self.inner.connect().await
    }
}

#[async_trait]
impl<A> BleStreamDriver for BleClient<A>
where
    A: BleClientDriver + BleStreamDriver + Send,
{
    async fn poll<'b>(&mut self, buffer: &'b mut [u8]) -> ockam::Result<BleEvent<'b>> {
        self.inner.poll(buffer).await
    }

    async fn write(&mut self, buffer: &[u8]) -> ockam::Result<()> {
        self.inner.write(buffer).await
    }
}

/// A BLE server that receives GATT commands and requests, and returns
/// responses to a BLE client.
pub struct BleServer<A>
where
    A: BleServerDriver + BleStreamDriver + Send,
{
    inner: A,
}

impl<A> BleServer<A>
where
    A: BleServerDriver + BleStreamDriver + Send,
{
    pub fn with_adapter(inner: A) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<A> BleServerDriver for BleServer<A>
where
    A: BleServerDriver + BleStreamDriver + Send,
{
    async fn bind(&mut self, ble_addr: &BleAddr) -> ockam::Result<()> {
        self.inner.bind(ble_addr).await
    }

    async fn start_advertising(&mut self) -> ockam::Result<()> {
        self.inner.start_advertising().await
    }
}

#[async_trait]
impl<A> BleStreamDriver for BleServer<A>
where
    A: BleServerDriver + BleStreamDriver + Send,
{
    async fn poll<'b>(&mut self, buffer: &'b mut [u8]) -> ockam::Result<BleEvent<'b>> {
        self.inner.poll(buffer).await
    }

    async fn write(&mut self, buffer: &[u8]) -> ockam::Result<()> {
        self.inner.write(buffer).await
    }
}
