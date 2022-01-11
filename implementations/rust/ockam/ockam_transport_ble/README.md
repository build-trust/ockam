# ockam_transport_ble

### Run:

    cargo build --example 04-routing-over-ble-transport-initiator

    cargo run --example 04-routing-over-ble-transport-initiator

    cargo run --example 05-secure-channel-over-ble-transport-initiator

----


## Transport Model

All Bluetooth Low Energy (BLE) devices use the Generic Attribute
Profile (GATT).

BLE transports will typically be based around GATT concepts and
feature the following terminology:

### Client

The device initiating GATT commands and requests. For example, a
computer or other device managing and collecting data from many
embedded BLE devices.

### Server

The device which receives GATT commands and requests. For example, a
small micro-controller collecting sensor data for transmission to a
Client device.

This can be confusing as we are used to the Client being the small
device and the Server being the big one!

### Characteristic

A data value transferred between client and server, for example, an
Ockam packet containing a sensor reading.

### Service

A collection of related characteristics, which operate together to
perform a particular function. For instance, the Ockam UART (Universal
Asynchronous Receiver/Transmitter) service contains the
characteristics required to implement the receive and transmit
channels.

### Descriptor

A descriptor provides additional information about a
characteristic. Descriptors are optional and each characteristic can
have any number of descriptors.

### Identifiers

Services, characteristics, and descriptors are collectively referred
to as attributes and are identified by UUIDs.

Any implementer may pick a random or pseudorandom UUID for proprietary
uses, but the Bluetooth SIG have reserved a [range of UUIDs
(xxxxxxxx-0000-1000-8000-00805F9B34FB)](https://www.bluetooth.com/specifications/assigned-numbers/)
for standard attributes.

#### InteroperaBLE identifier structure

    xxxxxxxx-xxxx-Mxxx-9xxx-xxxxxxxxxxxx
                  ^    ^-------------------- Upper bits must be 10_b to represent GUID Variant 1 (i.e. 8, 9, a or b)
                  |------------------------- Must be 4 to represent Version 4 - rest are random

#### Ockam Identifiers

Ockam Identifiers are in the range:

    xxxxxxxx-b19e-11e2-9e96-0800200c9a66

https://stackoverflow.com/questions/10867405/generating-v5-uuid-what-is-name-and-namespace
https://www.uuidtools.com/generate/v5
https://www.uuidtools.com/decode

---

## Assigned Identifiers

#### UART Service Identifier

    d973f2e0-b19e-11e2-9e96-0800200c9a66

#### UART Transmit Characteristic Identifier

    d973f2e1-b19e-11e2-9e96-0800200c9a66
    NOTIFY

#### UART Receive Characteristic Identifier

    d973f2e2-b19e-11e2-9e96-0800200c9a66
    WRITE_WITHOUT_RESPONSE | WRITE

---

## Mandatory identifier

TODO

* https://reelyactive.github.io/ble-identifier-reference.html
* https://reelyactive.github.io/diy/best-practices-ble-identifiers/

ockam -> 0ca?

---

# Setup notes

## Linux

### Dependencies

    apt-get install libdbus-1-dev libssl-dev

### dbus permissions

Edit `/etc/dbus-1/system.d/bluetooth.conf`:

    <policy user="antoine">
      <allow send_destination="org.bluez"/>
      <allow send_interface="org.bluez.Agent1"/>
      <allow send_interface="org.bluez.GattCharacteristic1"/>
      <allow send_interface="org.bluez.GattDescriptor1"/>
      <allow send_interface="org.freedesktop.DBus.ObjectManager"/>
      <allow send_interface="org.freedesktop.DBus.Properties"/>
    </policy>

## Mac

To use Bluetooth on macOS Big Sur (11) or later, you need to either package your binary into an application bundle with an `Info.plist` including `NSBluetoothAlwaysUsageDescription`, or (for a command-line application such as the examples included with `btleplug`) enable the Bluetooth permission for your terminal.

You can do the latter by going to:

    System Preferences → Security & Privacy → Privacy → Bluetooth

... clicking the '+' button, and selecting 'Terminal' (or iTerm or whichever terminal application you use).

Update: There is currently a bug in macOS Monterey that prevents Bluetooth Discovery for unsigned apps (even if you have given permissions to them)

To fix:

    Keychain Access => System Menu => Certificate Assistant => Create a Certificate...

        Name:             Self Signed Root
        Identity Type:    Self Signed Root
        Certificate Type: Code Signing

    codesign -f -o runtime --timestamp -s "Self Signed Root" target/debug/examples/04-routing-over-ble-transport-initiator
    codesign --entitlements Entitlements.plist -f -o runtime --timestamp -s "Self Signed Root" 04-routing-over-ble-transport-initiator
    codesign --entitlements Entitlements.plist -f -o runtime --timestamp -s "Apple Development: Antoine van Gelder (R972JJ8RXX)" 04-routing-over-ble-transport-initiator
    codesign -f -o runtime --timestamp -s "Developer ID Application: Antoine van Gelder (HLUFY5JD2L)" 04-routing-over-ble-transport-initiator


    codesign --entitlements Entitlements.plist -f -s "Apple Distribution" 04-routing-over-ble-transport-initiator
