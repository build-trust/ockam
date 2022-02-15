//! Driver for ST Micro BlueNRG Bluetooth radios
//!
//! Uses the [bluetooth-hci](https://crates.io/crates/bluetooth-hci)
//! and [bluenrg](https://crates.io/crates/bluetooth-hci) crates to
//! implement the BLE specification and chip-specific commands and
//! events for embedded targets.

#[cfg(feature = "atsame54")]
use atsame54_xpro as hal;
#[cfg(feature = "pic32")]
use pic32_hal as hal;
#[cfg(feature = "stm32f4")]
use stm32f4xx_hal as hal;
#[cfg(feature = "stm32h7")]
use stm32h7xx_hal as hal;

#[cfg(any(feature = "atsame54", feature = "pic32"))]
use embedded_hal;
#[cfg(any(feature = "stm32f4", feature = "stm32h7"))]
use hal::hal as embedded_hal;

#[cfg(not(feature = "atsame54"))]
use hal::time::MilliSeconds;
#[cfg(feature = "atsame54")]
use hal::time::Milliseconds as MilliSeconds;

mod ble_uart;

use core::fmt::Debug;
use embedded_hal::{
    blocking,
    digital::v2::{InputPin, OutputPin},
};
use hal::time::Hertz;
use nb::block;

use bluenrg::event::{BlueNRGEvent, GattAttributeModified};
use bluenrg::gatt::Commands;
use bluenrg::BlueNRG;
use bluetooth_hci::host::uart::Hci;

use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::io;

use crate::driver::BleEvent;
use crate::driver::{BleServerDriver, BleStreamDriver};
use crate::error::BleError;
use crate::BleAddr;

pub type Packet = bluetooth_hci::host::uart::Packet<BlueNRGEvent>;
pub type Event = bluetooth_hci::event::Event<BlueNRGEvent>;

#[derive(Debug, PartialEq)]
enum State {
    Disconnected,
    Advertising,
    Connected,
}

/// BleAdapter
pub struct BleAdapter<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError> {
    spi: SPI,
    bluetooth: BlueNRG<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    ble_context: ble_uart::BleContext,
    ble_addr: BleAddr,
    state: State,
}

/// BleAdapter implementation for bluenrg_ms devices
impl<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>
    BleAdapter<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    pub fn with_interface(
        spi: SPI,
        bluetooth: BlueNRG<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    ) -> Self {
        let ble_context = ble_uart::BleContext::default();
        Self {
            spi,
            bluetooth,
            ble_context,
            ble_addr: BleAddr::default(),
            state: State::Disconnected,
        }
    }

    pub fn reset<T, Time>(&mut self, timer: &mut T, time: Time) -> ockam::Result<()>
    where
        T: embedded_hal::timer::CountDown<Time = Time>,
        Time: Copy,
    {
        // hardware reset
        debug!("\n\treset bluenrg-ms device");
        self.bluetooth
            .reset(timer, time)
            .map_err(|_| BleError::HardwareError)?;
        self._handle_reset_response()
    }

    #[cfg(target_arch = "mips")]
    pub fn reset_with_delay<D, UXX>(&mut self, delay: &mut D, time: UXX) -> ockam::Result<()>
    where
        D: blocking::delay::DelayMs<UXX>,
        UXX: Copy,
    {
        // hardware reset
        debug!("\n\treset bluenrg-ms device");
        self.bluetooth
            .reset_with_delay(delay, time)
            .map_err(|_| BleError::HardwareError)?;
        self._handle_reset_response()
    }

    pub fn _handle_reset_response(&mut self) -> ockam::Result<()> {
        match self
            .bluetooth
            .with_spi(&mut self.spi, |controller| block!(controller.read()))
        {
            Ok(packet) => {
                let bluetooth_hci::host::uart::Packet::Event(event) = packet;
                match event {
                    Event::Vendor(BlueNRGEvent::HalInitialized(reason)) => (),
                    _ => {
                        error!("\t=> reset error: unknown event {:?}", event);
                        return Err(BleError::HardwareError.into());
                    }
                }
            }
            Err(e) => {
                error!("Device reset error: {:?}", e);
                return Err(BleError::HardwareError.into());
            }
        }

        // test device communication
        ble_uart::read_local_version_information(&mut self.spi, &mut self.bluetooth)
            .map_err(|_| BleError::HardwareError)?;

        Ok(())
    }
}

/// Implement trait: BleServerDriver
#[async_trait]
impl<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError> BleServerDriver
    for BleAdapter<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8> + Send,
    OutputPin1: OutputPin<Error = GpioError> + Send,
    OutputPin2: OutputPin<Error = GpioError> + Send,
    InputPin1: InputPin<Error = GpioError> + Send,
    SPI::Error: Debug,
    GpioError: Debug + Send,
{
    async fn bind(&mut self, ble_addr: &BleAddr) -> ockam::Result<()> {
        self.ble_addr = ble_addr.clone();

        ble_uart::setup(&mut self.spi, &mut self.bluetooth)
            .map_err(|_| BleError::ConfigurationFailed)?;

        crate::wait_ms!(500);

        self.ble_context =
            ble_uart::initialize_gatt_and_gap(&mut self.spi, &mut self.bluetooth, ble_addr)
                .map_err(|_| BleError::ConfigurationFailed)?;

        crate::wait_ms!(500);

        ble_uart::initialize_uart(&mut self.spi, &mut self.bluetooth, &mut self.ble_context)
            .map_err(|_| BleError::ConfigurationFailed)?;

        crate::wait_ms!(500);

        Ok(())
    }

    async fn start_advertising(&mut self) -> ockam::Result<()> {
        if let Err(e) = ble_uart::start_advertising(
            &mut self.spi,
            &mut self.bluetooth,
            &mut self.ble_context,
            &self.ble_addr,
        ) {
            debug!("BleAdapter::start_advertising error: {:?}", e);
            return Err(BleError::AdvertisingFailure.into());
        }
        Ok(())
    }
}

/// Implement trait: BleStreamDriver
#[async_trait]
impl<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError> BleStreamDriver
    for BleAdapter<'a, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8> + Send,
    OutputPin1: OutputPin<Error = GpioError> + Send,
    OutputPin2: OutputPin<Error = GpioError> + Send,
    InputPin1: InputPin<Error = GpioError> + Send,
    SPI::Error: Debug,
    GpioError: Debug + Send,
{
    async fn poll<'b>(&mut self, out_fragment: &'b mut [u8]) -> ockam::Result<BleEvent<'b>> {
        // avoid deadlocking the caller
        ockam_node::tokio::task::yield_now().await;

        match self.state {
            State::Disconnected => {
                debug!("BleStreamDriver::poll start advertising");
                self.start_advertising().await?;
                self.state = State::Advertising;
                debug!("\nstate = {:?}", self.state);
                #[cfg(feature = "debug_alloc")]
                ockam_executor::debug_alloc::stats();
            }
            State::Advertising => {}
            State::Connected => {}
        }

        let result = self
            .bluetooth
            .with_spi(&mut self.spi, |controller| controller.read());
        match result {
            Ok(Packet::Event(event)) => match event {
                Event::LeConnectionComplete(event) => {
                    debug!("\t=> LeConnectionComplete: -> {:?}", event);
                    self.ble_context.conn_handle = Some(event.conn_handle);
                    self.state = State::Connected;
                    debug!("\nstate = {:?}", self.state);
                    return Ok(BleEvent::ConnectionComplete);
                }

                Event::DisconnectionComplete(event) => {
                    debug!("\t=> DisconnectionComplete: -> {:?}", event);
                    self.ble_context.conn_handle = None;
                    self.state = State::Disconnected;
                    debug!("\nstate = {:?}", self.state);
                    return Ok(BleEvent::DisconnectionComplete);
                }

                Event::Vendor(BlueNRGEvent::GattAttributeModified(event)) => {
                    if event.attr_handle
                        == self
                            .ble_context
                            .uart_rx_attribute_handle
                            .expect("rx attribute handle is not set")
                    {
                        debug!(
                            "\t=> BlueNRGEvent::GattAttributeModified event: {:?}",
                            event.data().len()
                        );

                        let fragment = event.data();
                        let fragment_len = event.data().len();
                        if fragment_len > out_fragment.len() {
                            error!("response fragment too long");
                            return Err(BleError::ReadError.into());
                        }

                        let out_fragment = &mut out_fragment[..fragment_len];
                        out_fragment.copy_from_slice(&fragment[..fragment_len]);

                        return Ok(BleEvent::Received(out_fragment));
                    } else {
                        debug!("\t=> Rx unknown attribute: -> {:?}", event);
                        return Ok(BleEvent::None);
                    }
                }

                _ => {
                    warn!("\t=> unknown event: {:?}", event);
                    return Ok(BleEvent::None);
                }
            },

            Err(nb::Error::WouldBlock) => {
                return Ok(BleEvent::None);
            }

            Err(e) => {
                error!("controller read error: {:?}", e);
                return Err(BleError::ReadError.into());
            }
        }
    }

    async fn write(&mut self, buffer: &[u8]) -> ockam::Result<()> {
        debug!("BleAdapter<bluetooth_hci>::write");

        let Self {
            bluetooth,
            spi,
            ble_context,
            ..
        } = self;

        let result = block!(bluetooth.with_spi(spi, |controller| {
            controller.update_characteristic_value(
                &bluenrg::gatt::UpdateCharacteristicValueParameters {
                    service_handle: ble_context
                        .uart_service_handle
                        .expect("uart service handle has not been set"),
                    characteristic_handle: ble_context
                        .uart_tx_handle
                        .expect("uart tx handle has not been set"),
                    offset: 0x00,
                    value: &buffer,
                },
            )
        }));

        match result {
            Ok(()) => (),
            Err(e) => {
                error!("\t=> error writing data: {:?}", buffer);
                return Err(BleError::WriteError.into());
            }
        }

        Ok(())
    }
}

/// Map bluetooth_hci::host::Error to BleError
impl<E, VS> From<bluetooth_hci::host::Error<E, VS>> for BleError
where
    E: Debug,
    VS: Debug,
{
    fn from(e: bluetooth_hci::host::Error<E, VS>) -> Self {
        trace!("bluetooth_hci::host::Error error: {:?}", e);
        Self::HardwareError
    }
}

/// Map bluetooth_hci::host::uart::Error to BleError
impl<E, VS> From<bluetooth_hci::host::uart::Error<E, VS>> for BleError
where
    E: Debug,
    VS: Debug,
{
    fn from(e: bluetooth_hci::host::uart::Error<E, VS>) -> Self {
        trace!("bluetooth_hci::host::uart::Error error: {:?}", e);
        Self::HardwareError
    }
}

/// Map bluenrg::Error to BleError
impl<SpiError, GpioError> From<bluenrg::Error<SpiError, GpioError>> for BleError
where
    SpiError: Debug,
    GpioError: Debug,
{
    fn from(e: bluenrg::Error<SpiError, GpioError>) -> Self {
        trace!("bluenrg::Error error: {:?}", e);
        Self::HardwareError
    }
}

/// Map bluenrg::gap::Error to BleError
impl<E> From<bluenrg::gap::Error<E>> for BleError
where
    E: Debug,
{
    fn from(e: bluenrg::gap::Error<E>) -> Self {
        trace!("bluenrg::gap error: {:?}", e);
        Self::HardwareError
    }
}

/// Map bluenrg::gatt::Error to BleError
impl<E> From<bluenrg::gatt::Error<E>> for BleError
where
    E: Debug,
{
    fn from(e: bluenrg::gatt::Error<E>) -> Self {
        trace!("bluenrg::gatt error: {:?}", e);
        Self::HardwareError
    }
}

/// Map nb::Error to BleError
impl<GpioError> From<nb::Error<GpioError>> for BleError
where
    GpioError: Debug,
{
    fn from(e: nb::Error<GpioError>) -> Self {
        trace!("bluenrg::gatt error: {:?}", e);
        Self::HardwareError
    }
}

#[cfg(feature = "atsame54")]
/// get a unique hardware address for device
pub fn get_bd_addr() -> bluetooth_hci::BdAddr {
    let sn: [u8; 16] = atsame54_xpro::serial_number();
    let bytes: [u8; 6] = [
        sn[0] ^ sn[3] ^ sn[6] ^ sn[9] ^ sn[12] ^ sn[15],
        sn[1] ^ sn[4] ^ sn[7] ^ sn[10] ^ sn[13],
        sn[2] ^ sn[5] ^ sn[8] ^ sn[11] ^ sn[14],
        // https://www.adminsub.net/mac-address-finder/microchip
        0x39,
        0x80,
        0xD8,
    ];
    bluetooth_hci::BdAddr(bytes)
}

#[cfg(any(feature = "stm32f4", feature = "stm32h7"))]
/// get a unique hardware address for device
pub fn get_bd_addr() -> bluetooth_hci::BdAddr {
    let sn: &[u8; 12] = stm32_device_signature::device_id();
    let bytes: [u8; 6] = [
        sn[0] ^ sn[3] ^ sn[6] ^ sn[9],
        sn[1] ^ sn[4] ^ sn[7] ^ sn[10],
        sn[2] ^ sn[5] ^ sn[8] ^ sn[11],
        // https://www.adminsub.net/mac-address-finder/stmicroelectronics
        0xE1,
        0x80,
        0x00,
    ];
    bluetooth_hci::BdAddr(bytes)
}

#[cfg(feature = "pic32")]
/// get a unique hardware address for device
pub fn get_bd_addr() -> bluetooth_hci::BdAddr {
    let sn: &[u8; 12] = &[
        // TODO get serial number from processor
        01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12,
    ];
    let bytes: [u8; 6] = [
        sn[0] ^ sn[3] ^ sn[6] ^ sn[9],
        sn[1] ^ sn[4] ^ sn[7] ^ sn[10],
        sn[2] ^ sn[5] ^ sn[8] ^ sn[11],
        0x39,
        0x80,
        0xD8,
    ];
    bluetooth_hci::BdAddr(bytes)
}
