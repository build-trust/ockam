use embedded_hal::blocking;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use nb::{self, block};

use core::fmt::Debug;
use core::result::Result;
use core::time::Duration;

use bluetooth_hci::event::command::CommandComplete;
use bluetooth_hci::event::command::ReturnParameters as HciParams;
use bluetooth_hci::event::command::ReturnParameters::Vendor;
use bluetooth_hci::host::uart::Hci as _;
use bluetooth_hci::host::Hci as _;

use bluenrg::event::AttributeHandle;
use bluenrg::event::BlueNRGEvent;
use bluenrg::gap::{AuthenticationRequirements, DiscoverableParameters, OutOfBandAuthentication};
use bluenrg::gatt::{
    AddCharacteristicParameters, AddServiceParameters, CharacteristicEvent,
    CharacteristicPermission, CharacteristicProperty, EncryptionKeySize, ServiceType,
    UpdateCharacteristicValueParameters, Uuid, Uuid::Uuid128,
};
use bluenrg::hal::ConfigData;
use bluenrg::BlueNRG;
use bluenrg::OwnAddressType;

use bluenrg::event::command::ReturnParameters as Parameters;
use bluenrg::gap::Commands as GapCommands;
use bluenrg::gatt::Commands as GattCommands;
use bluenrg::hal::Commands as HalCommands;

use super::embedded_hal;
use crate::driver;
use crate::error::BleError;
use crate::BleAddr;

type Packet = bluetooth_hci::host::uart::Packet<BlueNRGEvent>;
pub type Event = bluetooth_hci::event::Event<BlueNRGEvent>;

/// BleContext
#[derive(Debug, Default)]
pub struct BleContext {
    pub service_handle: Option<bluenrg::gatt::ServiceHandle>,
    pub dev_name_handle: Option<bluenrg::gatt::CharacteristicHandle>,
    pub appearence_handle: Option<bluenrg::gatt::CharacteristicHandle>,

    pub uart_service_handle: Option<bluenrg::gatt::ServiceHandle>,
    pub uart_tx_handle: Option<bluenrg::gatt::CharacteristicHandle>,
    pub uart_rx_handle: Option<bluenrg::gatt::CharacteristicHandle>,
    pub uart_rx_attribute_handle: Option<AttributeHandle>,

    pub conn_handle: Option<bluenrg::event::ConnectionHandle>,
}

/// Currently we only support the BLE Peripheral role on embedded
pub const ROLE: bluenrg::gap::Role = bluenrg::gap::Role::PERIPHERAL;

/// BLE encryption key size
pub const BLE_ENCRYPTION_KEY_SIZE: usize = 16;

/// If `true`, the `AddCharacteristicParameters::characteristic_value_len`
/// parameter only takes 1 byte.
pub const FW_VERSION_BEFORE_V72: bool = true;

/// BLE Uart: Initial Setup (1/5)
pub fn setup<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("\tsoftware controller reset");
    block!(bluetooth.with_spi(spi, |controller| { controller.reset() }))?;

    // wait for reset to complete
    crate::wait_ms!(200);

    // read first reset response (Command complete)
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => debug!("\t=> controller reset: {:?}", event),
        Err(e) => {
            error!("controller reset error: {:?}", e);
            return Err(e.into());
        }
    }

    // read second reset response (HAL initialized)
    controller_read(spi, bluetooth)?;

    // configure public address
    let bd_addr = super::get_bd_addr();
    debug!("\twrite_config_data: public_address -> {:?}", bd_addr);
    block!(bluetooth.with_spi(spi, |controller| {
        controller.write_config_data(&ConfigData::public_address(bd_addr).build())
    }))?;
    controller_read(spi, bluetooth)?;

    // set power level
    debug!("\tset_tx_power_level");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.set_tx_power_level(bluenrg::hal::PowerLevel::Dbm8_0)
    }))?;
    controller_read(spi, bluetooth)?;

    Ok(())
}

/// BLE Uart: Initialize GATT and GAP (2/5)
pub fn initialize_gatt_and_gap<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    ble_addr: &BleAddr,
) -> Result<BleContext, BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("ble_uartinitialize_gatt_and_gap");

    // init gatt
    debug!("\tinit_gatt");
    block!(bluetooth.with_spi(spi, |controller| { controller.init_gatt() }))?;
    controller_read(spi, bluetooth)?;

    // ble_context
    let mut ble_context = BleContext::default();
    // TODO figure out why the bluenrg returns an invalid rx attribute handle
    ble_context.uart_rx_attribute_handle = Some(AttributeHandle(17));

    // init gap
    debug!("\tinit_gap");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.init_gap(ROLE, false, ble_addr.device_name.len() as u8)
    }))?;
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => match event {
            Event::CommandComplete(CommandComplete {
                return_params: Vendor(Parameters::GapInit(params)),
                ..
            }) => {
                debug!("\t=> CommandComplete::init_gap -> {:?}", params);
                ble_context.service_handle = Some(params.service_handle);
                ble_context.dev_name_handle = Some(params.dev_name_handle);
                ble_context.appearence_handle = Some(params.appearance_handle);
            }
            _ => debug!("\t=> unknown event: {:?}", event),
        },
        Err(e) => {
            error!("controller.reset error: {:?}", e);
            return Err(e.into());
        }
    }

    debug!("\tupdate_characteristic_value -> {:?}", ble_context);
    block!(bluetooth.with_spi(spi, |controller| {
        controller.update_characteristic_value(&UpdateCharacteristicValueParameters {
            service_handle: ble_context
                .service_handle
                .expect("service handle has not been set"),
            characteristic_handle: ble_context
                .dev_name_handle
                .expect("device name handdle has not been set"),
            offset: 0x00,
            value: ble_addr.device_name.as_bytes(),
        })
    }))?;
    controller_read(spi, bluetooth)?;

    Ok(ble_context)
}

/// BLE Uart: Initialize UART (3/5)
pub fn initialize_uart<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    ble_context: &mut BleContext,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("ble_uart::initialize_uart");

    // configure authorization requirements
    debug!("\tconfigure auth");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.set_authentication_requirement(&AuthenticationRequirements {
            mitm_protection_required: true,
            out_of_band_auth: OutOfBandAuthentication::Disabled,
            encryption_key_size_range: (7, 16),
            // we use a non-connectable mode so this pin is arbitrary
            fixed_pin: bluenrg::gap::Pin::Fixed(123456),
            bonding_required: true,
        })
    }))?;

    // wait for command to complete
    crate::wait_ms!(200);

    controller_read(spi, bluetooth)?;

    // add services
    if ROLE == bluenrg::gap::Role::PERIPHERAL {
        add_uart_service(spi, bluetooth, ble_context)?;
    }

    Ok(())
}

/// BLE Uart: Add UART service (4/5)
pub fn add_uart_service<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    ble_context: &mut BleContext,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("ble_uart::add_uart_service");

    // add uart service
    debug!("\tadd uart service");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.add_service(&AddServiceParameters {
            uuid: Uuid128(driver::uuid::SERVICE.to_le_bytes()),
            service_type: ServiceType::Primary,
            max_attribute_records: 9, // TODO
        })
    }))?;
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => match event {
            Event::CommandComplete(CommandComplete {
                return_params: Vendor(Parameters::GattAddService(service)),
                ..
            }) => {
                debug!("\t=> CommandComplete::add_service -> {:?}", service);
                ble_context.uart_service_handle = Some(service.service_handle)
            }
            _ => debug!("\t=> unknown event: {:?}", event),
        },
        Err(e) => {
            error!("controller.add_service error: {:?}", e);
            return Err(e.into());
        }
    }

    // add uart service tx characteristic: notify
    debug!("\tadd uart service tx characteristic: notify");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.add_characteristic(&AddCharacteristicParameters {
            service_handle: ble_context
                .uart_service_handle
                .expect("uart service handle has not been set"),
            characteristic_uuid: Uuid128(driver::uuid::WRITE.to_le_bytes()),
            characteristic_value_len: crate::driver::CHARACTERISTIC_VALUE_LENGTH,
            characteristic_properties: CharacteristicProperty::NOTIFY,

            // TODO https://github.com/danielgallagher0/bluenrg/pull/5
            // security_permissions: CharacteristicPermission::NONE,
            // gatt_event_mask: CharacteristicEvent::NONE,
            #[allow(unsafe_code)]
            security_permissions: unsafe { core::mem::transmute(0_u8) },
            #[allow(unsafe_code)]
            gatt_event_mask: unsafe { core::mem::transmute(0_u8) },

            encryption_key_size: EncryptionKeySize::with_value(BLE_ENCRYPTION_KEY_SIZE)
                .expect("wrong size encryption key"),
            is_variable: true,
            fw_version_before_v72: FW_VERSION_BEFORE_V72,
        })
    }))?;
    ble_context.uart_tx_handle = handle_event(spi, bluetooth, |event| match event {
        Event::CommandComplete(CommandComplete {
            return_params: Vendor(Parameters::GattAddCharacteristic(characteristic)),
            ..
        }) => {
            debug!(
                "\t=> CommandComplete::add_characteristic -> {:?}",
                characteristic
            );
            Some(characteristic.characteristic_handle)
        }
        _ => {
            debug!("\t=> unknown event: {:?}", event);
            None
        }
    });

    // add uart service rx characteristic: write
    debug!("\tadd uart service rx characteristic: write");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.add_characteristic(&AddCharacteristicParameters {
            service_handle: ble_context
                .uart_service_handle
                .expect("uart service handle has not been set"),
            characteristic_uuid: Uuid128(driver::uuid::READ.to_le_bytes()),
            characteristic_value_len: crate::driver::CHARACTERISTIC_VALUE_LENGTH,
            characteristic_properties: CharacteristicProperty::WRITE
                | CharacteristicProperty::WRITE_WITHOUT_RESPONSE,

            // TODO https://github.com/danielgallagher0/bluenrg/pull/5
            // security_permissions: CharacteristicPermission::NONE,
            #[allow(unsafe_code)]
            security_permissions: unsafe { core::mem::transmute(0_u8) },

            gatt_event_mask: CharacteristicEvent::ATTRIBUTE_WRITE,
            encryption_key_size: EncryptionKeySize::with_value(BLE_ENCRYPTION_KEY_SIZE)
                .expect("wrong size encryption key"),
            is_variable: true,
            fw_version_before_v72: FW_VERSION_BEFORE_V72,
        })
    }))?;
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => match event {
            Event::CommandComplete(CommandComplete {
                return_params: Vendor(Parameters::GattAddCharacteristic(characteristic)),
                ..
            }) => {
                debug!(
                    "\t=> CommandComplete::add_characteristic -> {:?}",
                    characteristic
                );
                ble_context.uart_rx_handle = Some(characteristic.characteristic_handle);
            }
            _ => debug!("\t=> unknown event: {:?}", event),
        },
        Err(e) => {
            error!("controller.add_characteristic error: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// BLE Uart: Start advertising (5/5)
pub fn start_advertising<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    _context: &mut BleContext,
    ble_addr: &BleAddr,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("ble_uart::start_advertising");

    debug!("\tle_set_scan_response_data (disable scan response)");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.le_set_scan_response_data(&[])
    }))?;
    controller_read(spi, bluetooth)?;

    debug!("\tset_discoverable (put the device in a non-connectable mode)");
    block!(bluetooth.with_spi(spi, |controller| {
        controller.set_discoverable(&DiscoverableParameters {
            advertising_type: bluenrg::AdvertisingType::ConnectableUndirected,
            advertising_interval: Some((Duration::from_millis(250), Duration::from_millis(1000))),
            address_type: OwnAddressType::Public,
            filter_policy: bluenrg::AdvertisingFilterPolicy::AllowConnectionAndScan,
            local_name: Some(bluenrg::gap::LocalName::Shortened(
                ble_addr.local_name.as_bytes(),
            )),
            advertising_data: &[],
            conn_interval: (None, None),
        })
    }))?;
    controller_read(spi, bluetooth)?;

    // delete some ad types to make space
    debug!("\tdelete_ad_type");

    block!(bluetooth.with_spi(spi, |controller| {
        controller.delete_ad_type(bluenrg::gap::AdvertisingDataType::TxPowerLevel)
    }))?;
    controller_read(spi, bluetooth)?;

    block!(bluetooth.with_spi(spi, |controller| {
        controller.delete_ad_type(bluenrg::gap::AdvertisingDataType::PeripheralConnectionInterval)
    }))?;
    controller_read(spi, bluetooth)?;

    Ok(())
}

/// Generic event handler that takes a callback function
pub fn handle_event<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError, F, T>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
    handler: F,
) -> Option<T>
where
    F: FnOnce(&Event) -> Option<T>,
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => handler(&event),
        Err(e) => {
            error!("error handling event: {:?}", e);
            None
        }
    }
}

/// Generic controller read for commands that do not return a value
pub fn controller_read<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin1, GpioError>,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin1: InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    match block!(bluetooth.with_spi(spi, |controller| controller.read())) {
        Ok(Packet::Event(event)) => match event {
            Event::ConnectionComplete(params) => {
                debug!("\t=> ConnectionComplete -> {:?}", params);
            }
            Event::Vendor(BlueNRGEvent::HalInitialized(reason)) => {
                debug!("\t=> Vendor::HalInitialized -> {:?}", reason);
            }
            _ => {
                debug!("\t=> unknown event -> {:?}", event);
            }
        },
        Err(e) => {
            return Err(e.into());
        }
    }
    Ok(())
}

/// Read local version information of device
pub fn read_local_version_information<'buf, SPI, OutputPin1, OutputPin2, InputPin, GpioError>(
    spi: &mut SPI,
    bluetooth: &mut BlueNRG<'buf, SPI, OutputPin1, OutputPin2, InputPin, GpioError>,
) -> Result<(), BleError>
where
    SPI: blocking::spi::transfer::Default<u8> + blocking::spi::write::Default<u8>,
    OutputPin1: OutputPin<Error = GpioError>,
    OutputPin2: OutputPin<Error = GpioError>,
    InputPin: embedded_hal::digital::v2::InputPin<Error = GpioError>,
    SPI::Error: Debug,
    GpioError: Debug,
{
    debug!("\tread bluenrg-ms local version information");
    bluetooth.with_spi(spi, |controller| {
        block!(controller.read_local_version_information())
    })?;
    match bluetooth.with_spi(spi, |controller| block!(controller.read())) {
        Ok(packet) => {
            let bluetooth_hci::host::uart::Packet::Event::<BlueNRGEvent>(event) = packet;
            match event {
                Event::CommandComplete(CommandComplete {
                    return_params: HciParams::ReadLocalVersionInformation(version),
                    ..
                }) => {
                    debug!(
                        "\t=> CommandComplete::read_local_version_information -> {:?}",
                        version
                    );
                    return Ok(());
                }
                _ => {
                    error!(
                        "\tread_local_version_information received an unknown event: {:?}",
                        event
                    );
                    return Err(BleError::HardwareError);
                }
            }
        }
        Err(e) => {
            error!("read_local_version_information error: {:?}", e);
            return Err(e.into());
        }
    }
}
