// This file is based on [embassy's device](https://github.com/embassy-rs/embassy/blob/3396a519389837727c17eae2ebf65d5c93f70551/embassy-net/src/device.rs)
use ockam_core::compat::task::Waker;
use smoltcp::phy::Device as SmolDevice;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::Result;

use ockam_core::compat::sync::Mutex;

use packet_pool::PacketBoxExt;
use packet_pool::{Packet, PacketBox, PacketBuf};

mod packet_pool;

// Tap devices only make sense in std
#[cfg(feature = "std")]
mod tuntap;

// Tap devices only make sense in std
#[cfg(feature = "std")]
pub use tuntap::TunTapDevice;

// This Device is very similar to Smoltcp's device however it makes the need
// to handle `Waker` explicit
/// Similar to [smoltcp::phy::Device] but allows passing a `Waker` explicitly.
pub trait Device {
    /// Returns whether the device is prepared to start sending packages.
    fn is_transmit_ready(&mut self) -> bool;
    /// Transmits a package, if the buffer is full the `waker` should be signaled once the device is available to send again.
    fn transmit(&mut self, pkt: PacketBuf, waker: &Option<Waker>);
    /// Recieves a package, if no package is available yet the `waker` should be signaled once the package is ready.
    fn receive(&mut self, waker: &Option<Waker>) -> Option<PacketBuf>;
    /// Returns the device's [DeviceCapabilities]
    fn capabilities(&mut self) -> DeviceCapabilities;
}

pub struct DeviceAdapter {
    // Note: This Mutex is not strictly necessary since `DeviceAdapter will always sit behind a `Stack` which is hold behind a Mutex.
    // However, we do need to mutate `device` which would require unsafe so to keep it simple for now we will keep the `Mutex`.
    pub device: &'static Mutex<(dyn Device + Send)>,
    caps: DeviceCapabilities,
    waker: Option<Waker>,
}

impl DeviceAdapter {
    pub(crate) fn new(device: &'static Mutex<(dyn Device + Send)>) -> Self {
        let caps = device.lock().unwrap().capabilities();
        Self {
            caps,
            device,
            waker: None,
        }
    }

    pub(crate) fn get_waker(&self) -> &Option<Waker> {
        &self.waker
    }

    pub(crate) fn register_waker(&mut self, waker: &Waker) {
        match self.waker {
            // Optimization: If both the old and new Wakers wake the same task, we can simply
            // keep the old waker, skipping the clone. (In most executor implementations,
            // cloning a waker is somewhat expensive, comparable to cloning an Arc).
            Some(ref w2) if (w2.will_wake(waker)) => {}
            _ => {
                // clone the new waker and store it
                if let Some(old_waker) = core::mem::replace(&mut self.waker, Some(waker.clone())) {
                    // We had a waker registered for another task. Wake it, so the other task can
                    // reregister itself if it's still interested.
                    //
                    // If two tasks are waiting on the same thing concurrently, this will cause them
                    // to wake each other in a loop fighting over this WakerRegistration. This wastes
                    // CPU but things will still work.
                    //
                    // If the user wants to have two tasks waiting on the same thing they should use
                    // a more appropriate primitive that can store multiple wakers.
                    old_waker.wake()
                }
            }
        }
    }
}

impl<'a> SmolDevice<'a> for DeviceAdapter {
    type RxToken = RxToken;
    type TxToken = TxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let tx_pkt = PacketBox::new(Packet::new())?;
        let rx_pkt = Mutex::lock(self.device).unwrap().receive(&self.waker)?;
        let rx_token = RxToken { pkt: rx_pkt };
        let tx_token = TxToken {
            device: self.device,
            pkt: tx_pkt,
            waker: &self.waker,
        };

        Some((rx_token, tx_token))
    }

    /// Construct a transmit token.
    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        if !self.device.lock().unwrap().is_transmit_ready() {
            return None;
        }

        let tx_pkt = PacketBox::new(Packet::new())?;
        Some(TxToken {
            device: self.device,
            pkt: tx_pkt,
            waker: &self.waker,
        })
    }

    /// Get a description of device capabilities.
    fn capabilities(&self) -> DeviceCapabilities {
        self.caps.clone()
    }
}

pub struct RxToken {
    pkt: PacketBuf,
}

impl smoltcp::phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: SmolInstant, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        f(&mut self.pkt)
    }
}

pub struct TxToken<'a> {
    device: &'static Mutex<(dyn Device + Send)>,
    pkt: PacketBox,
    waker: &'a Option<Waker>,
}

impl<'a> smoltcp::phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, _timestamp: SmolInstant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buf = self.pkt.slice(0..len);
        let r = f(&mut buf)?;
        self.device.lock().unwrap().transmit(buf, self.waker);
        Ok(r)
    }
}
