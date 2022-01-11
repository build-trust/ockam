//! Driver for OS targets
//!
//! Uses the [btleplug](https://crates.io/crates/btleplug/) crate to
//! provide a cross-platform interface to BLE API's on Linux, macOS
//! and Windows.

use core::pin::Pin;

use btleplug::api::{Central, Manager as _, Peripheral};
use btleplug::api::{CharPropFlags, Characteristic, ValueNotification};
use btleplug::platform::{Adapter, Manager};
use futures::stream::{Stream, StreamExt};
use uuid::Uuid;

use ockam_core::async_trait;

use crate::driver::BleEvent;
use crate::driver::{BleClientDriver, BleStreamDriver};
use crate::error::BleError;
use crate::BleAddr;

/// UUID's
pub const UUID: Uuid = Uuid::from_u128_le(0x669a0c20_0008_969e_e211_9eb1e0f273d9);
pub const RX_UUID: Uuid = Uuid::from_u128_le(0x669a0c20_0008_969e_e211_9eb1e1f273d9);
pub const TX_UUID: Uuid = Uuid::from_u128_le(0x669a0c20_0008_969e_e211_9eb1e2f273d9);

/// Convert btleplug::Error to BleError
impl From<btleplug::Error> for BleError {
    fn from(e: btleplug::Error) -> BleError {
        trace!("From<btleplug::Error> -> {:?}", e);
        match e {
            btleplug::Error::PermissionDenied => BleError::PermissionDenied,
            btleplug::Error::DeviceNotFound => BleError::NotFound,
            btleplug::Error::NotConnected => BleError::NotConnected,
            btleplug::Error::NotSupported(_) => BleError::NotSupported,
            btleplug::Error::TimedOut(_) => BleError::TimedOut,
            btleplug::Error::Uuid(_) => BleError::ConfigurationFailed,
            btleplug::Error::InvalidBDAddr(_) => BleError::ConfigurationFailed,
            btleplug::Error::Other(e) => BleError::Other,
        }
    }
}

/// BleAdaptor
pub struct BleAdapter {
    manager: Manager,
    peripheral: Option<btleplug::platform::Peripheral>,
    rx: Option<Characteristic>,
    tx: Option<Characteristic>,

    notification_stream: Option<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>>,
}

impl BleAdapter {
    pub async fn try_new() -> ockam::Result<BleAdapter> {
        let manager = Manager::new().await.map_err(BleError::from)?;
        Ok(Self {
            manager,
            peripheral: None,
            rx: None,
            tx: None,
            notification_stream: None,
        })
    }
}

#[async_trait]
impl BleClientDriver for BleAdapter {
    async fn scan(&mut self, ble_addr: &BleAddr) -> ockam::Result<()> {
        let adapters = self.manager.adapters().await.map_err(BleError::from)?;
        if adapters.is_empty() {
            error!("No Bluetooth adapters found");
            return Err(BleError::HardwareError.into());
        }
        debug!("BleAdapter::scan scanning adapters: {:?}", adapters.len());

        let mut retry_count = 0;
        let local_name_filter = ble_addr.to_string();
        self.peripheral = loop {
            match scan_for_peripheral_name(&adapters, &local_name_filter).await {
                Ok(peripheral) => break Some(peripheral),
                Err(e) => {
                    warn!("Could not find peripheral, resuming scan: {:?}", e);
                }
            }
            retry_count += 1;
            if retry_count > 20 {
                warn!(
                    "Could not find peripheral, aborting scan after {} retries",
                    retry_count
                );
                return Err(BleError::NotFound.into());
            }
        };

        Ok(())
    }

    async fn connect(&mut self) -> ockam::Result<()> {
        if self.peripheral.is_none() {
            return Err(BleError::NotFound.into());
        }

        let peripheral = self.peripheral.as_mut().unwrap();
        let properties = peripheral.properties().await.map_err(BleError::from)?;
        let is_connected = peripheral.is_connected().await.map_err(BleError::from)?;
        let local_name = properties
            .unwrap()
            .local_name
            .unwrap_or_else(|| String::from("(peripheral name unknown)"));
        debug!("Found matching peripheral {:?}...", local_name);

        if !is_connected {
            if let Err(err) = peripheral.connect().await {
                warn!(
                    "Error connecting to peripheral, continuing: {}",
                    err.to_string()
                );
            } else {
                debug!("Connected to peripheral");
            }
        } else {
            debug!("Already connected");
        }
        let is_connected = peripheral.is_connected().await.map_err(BleError::from)?;
        debug!(
            "Peripheral: {:?} connection status: {:?}",
            local_name, is_connected
        );

        // discover characteristics
        peripheral
            .discover_services()
            .await
            .map_err(BleError::from)?;
        debug!("Discover peripheral {:?} services...", &local_name);
        for service in peripheral.services() {
            trace!(
                "Service UUID {}, primary: {}",
                service.uuid,
                service.primary
            );
            for characteristic in service.characteristics {
                trace!("  {:?}", characteristic);
                if characteristic.uuid == RX_UUID
                    && characteristic.properties.contains(CharPropFlags::NOTIFY)
                {
                    self.rx = Some(characteristic);
                } else if characteristic.uuid == TX_UUID
                    && characteristic.properties.contains(CharPropFlags::WRITE)
                {
                    self.tx = Some(characteristic);
                }
            }
        }

        if self.rx.is_none() || self.tx.is_none() {
            debug!("No compatible devices found");
            return Err(BleError::NotSupported.into());
        }

        // subscribe to notifications
        peripheral
            .subscribe(self.rx.as_ref().unwrap())
            .await
            .map_err(BleError::from)?;
        let notification_stream = peripheral.notifications().await.map_err(BleError::from)?;
        self.notification_stream = Some(notification_stream);

        Ok(())
    }
}

#[async_trait]
impl BleStreamDriver for BleAdapter {
    async fn poll<'b>(&mut self, buffer: &'b mut [u8]) -> ockam::Result<BleEvent<'b>> {
        // avoid deadlocking the caller
        ockam_node::tokio::task::yield_now().await;

        if self.peripheral.is_none() {
            return Err(BleError::NotConnected.into());
        }

        let mut rx_stream = self.notification_stream.as_mut().unwrap();
        let waker = futures::task::noop_waker();
        let mut context = core::task::Context::from_waker(&waker);

        if let core::task::Poll::Ready(Some(item)) = rx_stream.poll_next_unpin(&mut context) {
            match item.uuid {
                RX_UUID => {
                    trace!("\t=> Rx: -> {:?}", item);
                    let data = item.value;
                    let len = core::cmp::min(data.len(), buffer.len() - 1);
                    buffer[..len].copy_from_slice(&data[..len]);
                    return Ok(BleEvent::Received(&buffer[..len]));
                }
                _ => {
                    warn!("[btleplug]\t=> Rx unknown characteristic: -> {:?}", item);
                    //return Err(BleError::NotSupported.into());
                    return Ok(BleEvent::None);
                }
            }
        }

        Ok(BleEvent::None)
    }

    async fn write(&mut self, buffer: &[u8]) -> ockam::Result<()> {
        trace!("write {} bytes", buffer.len());

        if self.peripheral.is_none() {
            error!("No peripheral found");
            return Err(BleError::NotConnected.into());
        } else if self.rx.is_none() || self.tx.is_none() {
            error!("No compatible devices found");
            return Err(BleError::NotSupported.into());
        }

        let peripheral = self.peripheral.as_ref().unwrap();
        let tx = self.tx.as_ref().unwrap();

        let result = peripheral
            .write(tx, buffer, btleplug::api::WriteType::WithoutResponse)
            .await;

        match result {
            Err(e) => {
                error!("Error writing data: {:?}", e);
            }
            Ok(()) => {
                trace!("Success writing data: {:?}", buffer);
            }
        }

        Ok(())
    }
}

async fn scan_for_peripheral_name(
    adapters: &[Adapter],
    local_name_filter: &str,
) -> ockam::Result<btleplug::platform::Peripheral> {
    for (count, adapter) in adapters.iter().enumerate() {
        let peripherals = adapter.peripherals().await.map_err(BleError::from)?;
        debug!(
            "Scanning adapter {}: {} peripherals",
            count,
            peripherals.len()
        );
        let mut scan_filter = btleplug::api::ScanFilter::default();
        match adapter.start_scan(scan_filter).await {
            Ok(_) => (),
            Err(e) => {
                warn!(
                    "Can't scan BLE adapter for connected devices: {:?}",
                    adapter
                );
                continue;
            }
        }

        crate::wait_ms!(5_000);

        adapter.stop_scan().await.map_err(BleError::from)?;

        let peripherals = match adapter.peripherals().await {
            Ok(peripherals) => peripherals,
            Err(e) => {
                warn!("No BLE peripherals found on adapter: {:?}", adapter);
                continue;
            }
        };
        if peripherals.is_empty() {
            warn!("No BLE peripherals found on adapter: {:?}", adapter);
            continue;
        }

        for peripheral in peripherals {
            let properties = peripheral.properties().await.map_err(BleError::from)?;
            let local_name = properties
                .unwrap()
                .local_name
                .unwrap_or_else(|| String::from("(peripheral name unknown)"));

            // check if it's the peripheral we want.
            if local_name.contains(local_name_filter) {
                let is_connected = peripheral.is_connected().await.map_err(BleError::from)?;

                debug!(
                    "Found peripheral: {:?}\tconnected: {:?}\n",
                    &local_name, is_connected
                );
                return Ok(peripheral);
            }
        }
    }

    Err(BleError::NotFound.into())
}
