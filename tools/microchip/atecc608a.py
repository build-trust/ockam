"""
Basic Configuration Common Use Cases
"""
# (c) 2015-2018 Microchip Technology Inc. and its subsidiaries.
#
# Subject to your compliance with these terms, you may use Microchip software
# and any derivatives exclusively with Microchip products. It is your
# responsibility to comply with third party license terms applicable to your
# use of third party software (including open source software) that may
# accompany Microchip software.
#
# THIS SOFTWARE IS SUPPLIED BY MICROCHIP "AS IS". NO WARRANTIES, WHETHER
# EXPRESS, IMPLIED OR STATUTORY, APPLY TO THIS SOFTWARE, INCLUDING ANY IMPLIED
# WARRANTIES OF NON-INFRINGEMENT, MERCHANTABILITY, AND FITNESS FOR A
# PARTICULAR PURPOSE. IN NO EVENT WILL MICROCHIP BE LIABLE FOR ANY INDIRECT,
# SPECIAL, PUNITIVE, INCIDENTAL OR CONSEQUENTIAL LOSS, DAMAGE, COST OR EXPENSE
# OF ANY KIND WHATSOEVER RELATED TO THE SOFTWARE, HOWEVER CAUSED, EVEN IF
# MICROCHIP HAS BEEN ADVISED OF THE POSSIBILITY OR THE DAMAGES ARE
# FORESEEABLE. TO THE FULLEST EXTENT ALLOWED BY LAW, MICROCHIP'S TOTAL
# LIABILITY ON ALL CLAIMS IN ANY WAY RELATED TO THIS SOFTWARE WILL NOT EXCEED
# THE AMOUNT OF FEES, IF ANY, THAT YOU HAVE PAID DIRECTLY TO MICROCHIP FOR
# THIS SOFTWARE.
​
from cryptoauthlib import *
from cryptoauthlib.device import *
from common import *
import time
import ctypes
import base64
​
# Example configuration for ATECC608A minus the first 16 bytes which are fixed by the factory
_atecc608_config = bytearray.fromhex(
    'C0'                                                # 16        : I2C Address - Leave at C0
    '00'                                                # 17        : Reserved - must be 0
    '00'                                                # 18        : Count Match - Set to 0 to disable
    '00'                                                # 19        : ChipMode - No clock divider/defaults for watchdog, ttl and i2c
    '84 20'                                             # 20-21     : Slot 0 - ECC priv. No write. ECDH encrypted output.
    '84 20'                                             # 22-23     : Slot 1 - ECC priv. No write. ECDH encrypted output.
    '84 20'                                             # 24-25     : Slot 2 - ECC priv. No write. ECDH encrypted output.
    '84 67'                                             # 26-27     : Slot 3 - ECC priv. Encrypt write via Slot 7. ECDH encrypted output.
    '84 67'                                             # 28-29     : Slot 4 - ECC priv. Encrypt write via Slot 7. ECDH encrypted output.
    '84 67'                                             # 30-31     : Slot 5 - ECC priv. Encrypt write via Slot 7. ECDH encrypted output.
    '8F 0F'                                             # 32-33     : Slot 6 - IO Protection Key
    '8F 0F'                                             # 34-35     : Slot 7 - Write key for slots 3-5. Read key for slots 8-10.
    'C7 0F'                                             # 36-37     : Slot 8 - General purpose slot. Allows encrypted read via slot 7.
    'C7 0F'                                             # 38-39     : Slot 9 - General purpose slot. Allows encrypted read via slot 7.
    'C7 0F'                                             # 40-41     : Slot 10 - General purpose slot. Allows encrypted read via slot 7.
    '0F 0F'                                             # 42-43     : Slot 11 - General purpose slot. Allows plaintext read.
    '0F 0F'                                             # 44-45     : Slot 12 - General purpose slot. Allows plaintext read.
    '8F 0F'                                             # 46-47     : Slot 13 - AES Key Slot. Write only, no read.
    '8F 0F'                                             # 48-49     : Slot 14 - AES Key Slot. Write only, no read.
    '8F 0F'                                             # 50-51     : Slot 15 - AES Key Slot. Write only, no read.
    'FF FF FF FF 00 00 00 00'                           # 52-59     : Counter<0>
    'FF FF FF FF 00 00 00 00'                           # 60-67     : Counter<1>
    '00'                                                # 68        : UseLock - Disabled
    '00'                                                # 69        : Volatile - Disabled
    '00 00'                                             # 70-71     : Secure Boot - Disabled
    'F0'                                                # 72        : KdfIvLoc - Disabled
    'FF FF'                                             # 73-74     : KdfIvStr - Disabled
    'FF FF FF FF FF FF FF FF FF'                        # 75-83     : Reserved - Must be zero
    '00'                                                # 84        : UserExtra
    '00'                                                # 85        : UserExtraAdd
    '55'                                                # 86        : Lock Value - Set via Lock Command
    '55'                                                # 87        : LockConfig - Set via Lock Command
    'FF FF'                                             # 88-89     : SlotLocked - Default to every slot unlocked
    '01 60'                                             # 90-91     : ChipOptions
    '00 00 00 00'                                       # 92-95     : X509Format
    '73 00'                                             # 96-97     : Key 0 - ECC Key, requires random nonce, lockable
    '53 00'                                             # 98-99     : Key 1 - ECC Key, requires random nonce
    '53 00'                                             # 100-101   : Key 2 - ECC Key, requires random nonce
    '73 00'                                             # 102-103   : Key 3 - ECC Key, requires random nonce, lockable
    '53 00'                                             # 104-105   : Key 4 - ECC Key, requires random nonce
    '53 00'                                             # 106-107   : Key 5 - ECC Key, requires random nonce
    '3C 00'                                             # 108-109   : Key 6 - IO Protection Key, lockable
    '3C 00'                                             # 110-111   : Key 7 - Write key, not ECC, lockable.
    '1C 00'                                             # 112-113   : Key 8 - Data slot.
    '1C 00'                                             # 114-115   : Key 9 - Data slot.
    '1C 00'                                             # 116-117   : Key 10 - Data slot.
    '1C 00'                                             # 118-119   : Key 11 - Data slot.
    '1C 00'                                             # 120-121   : Key 12 - Data slot.
    '3A 00'                                             # 122-123   : Key 13 - AES Key, lockable
    '3A 00'                                             # 124-125   : Key 14 - AES Key, lockable
    '3A 00'                                             # 126-127   : Key 15 - AES Key, lockable
    )
​
_configs = {'ATECC608A': _atecc608_config}
​
# Safe input if using python 2
try: input = raw_input
except NameError: pass
​
​
def configure_device(iface='hid', device='ecc', i2c_addr=None, keygen=True, **kwargs):
    ATCA_SUCCESS = 0x00
​
    # Loading cryptoauthlib(python specific)
    load_cryptoauthlib()
​
    # Get the target default config
    cfg = eval('cfg_at{}a_{}_default()'.format(atca_names_map.get(device), atca_names_map.get(iface)))
​
    # Set interface parameters
    if kwargs is not None:
        for k, v in kwargs.items():
            icfg = getattr(cfg.cfg, 'atca{}'.format(iface))
            setattr(icfg, k, int(v, 16))
​
    # Basic Raspberry Pi I2C check
    if 'i2c' == iface and check_if_rpi():
        cfg.cfg.atcai2c.bus = 1
​
    # Initialize the stack
    assert atcab_init(cfg) == ATCA_SUCCESS
    print('')
​
    # Check device type
    info = bytearray(4)
    assert atcab_info(info) == ATCA_SUCCESS
    dev_name = get_device_name(info)
    dev_type = get_device_type_id(dev_name)
​
    # Reinitialize if the device type doesn't match the default
    if dev_type != cfg.devtype:
        cfg.dev_type = dev_type
        assert atcab_release() == ATCA_SUCCESS
        time.sleep(1)
        assert atcab_init(cfg) == ATCA_SUCCESS
​
    # Request the Serial Number
    serial_number = bytearray(9)
    assert atcab_read_serial_number(serial_number) == ATCA_SUCCESS
    print('\nSerial number: ')
    print(pretty_print_hex(serial_number, indent='    '))
​
    # Check the zone locks
    print('\nReading the Lock Status')
    is_locked = AtcaReference(False)
    assert ATCA_SUCCESS == atcab_is_locked(0, is_locked)
    config_zone_lock = bool(is_locked.value)
​
    assert ATCA_SUCCESS == atcab_is_locked(1, is_locked)
    data_zone_lock = bool(is_locked.value)
​
    print('    Config Zone: {}'.format('Locked' if config_zone_lock else 'Unlocked'))
    print('    Data Zone: {}'.format('Locked' if data_zone_lock else 'Unlocked'))
​
    # Get Current I2C Address
    print('\nGetting the I2C Address')
    response = bytearray(4)
    assert ATCA_SUCCESS == atcab_read_bytes_zone(0, 0, 16, response, 4)
    print('    Current Address: {:02X}'.format(response[0]))
​
    # Program the configuration zone
    print('\nProgram Configuration')
    if not config_zone_lock:
        config = _configs.get(dev_name)
        if config is None:
            raise ValueError('Unknown Device Type: {}'.format(dev_type))
​
        # Update with the target I2C Address
        if i2c_addr is not None:
            config[0] = i2c_addr
​
        print('\n    New Address: {:02X}'.format(config[0]))
        ck590_i2c_addr = 0xC0 if dev_name != 'ATSHA204A' else 0xC8
        if config[0] != ck590_i2c_addr:
            print('    The AT88CK590 Kit does not support changing the I2C addresses of devices.')
            print('    If you are not using an AT88CK590 kit you may continue without errors')
            print('    otherwise exit and specify a compatible (0x{:02X}) address.'.format(ck590_i2c_addr))
            if 'Y' != input('    Continue (Y/n): '):
                exit(0)
​
        print('    Programming {} Configuration'.format(dev_name))
​
        # Write configuration
        assert ATCA_SUCCESS == atcab_write_bytes_zone(0, 0, 16, config, len(config))
        print('        Success')
​
        # Verify Config Zone
        print('    Verifying Configuration')
        config_qa = bytearray(len(config))
        atcab_read_bytes_zone(0, 0, 16, config_qa, len(config_qa))
​
        if config_qa != config:
            raise ValueError('Configuration read from the device does not match')
        print('        Success')
​
        print('    Locking Configuration')
        assert ATCA_SUCCESS == atcab_lock_config_zone()
        print('        Locked')
    else:
        print('    Locked, skipping')
    
    # Check data zone lock
    print('\nActivating Configuration')
    if not data_zone_lock:
        # Generate initial ECC key pairs, if applicable
        key_gen(dev_name)
​
        # Lock the data zone
        assert ATCA_SUCCESS == atcab_lock_data_zone()
        print('    Activated')
    else:
        print('    Already Active')
​
    # Generate new keys
    if keygen and data_zone_lock:
        print('\nGenerating New Keys')
        key_gen(dev_name)
​
    atcab_release()
​
​
def key_gen(dev_name):
    """Reviews the configuration of a device and generates new random ECC key pairs for slots that allow it."""
    ATCA_SUCCESS = 0x00
​
    if 'ECC' not in dev_name:
        return  # SHA device, no keys to generate
​
    # Read the device configuration
    config_data = bytearray(128)
    assert ATCA_SUCCESS == atcab_read_config_zone(config_data)
    if dev_name == 'ATECC508A':
        config = Atecc508aConfig.from_buffer(config_data)
    elif dev_name == 'ATECC608A':
        config = Atecc608aConfig.from_buffer(config_data)
    else:
        raise ValueError('Unsupported device {}'.format(dev_name))
​
    # Review all slot configurations and generate keys where possible
    for slot in range(16):
        if not config.KeyConfig[slot].Private:
            continue  # Not a private key
        if config.LockValue != 0x55:
            # Data zone is already locked, additional conditions apply
            skip_msg = '    Skipping key pair generation in slot {}: '.format(slot)
            if not config.SlotConfig[slot].WriteConfig & 0x02:
                print(skip_msg + 'GenKey is disabled')
                continue
            if not config.SlotLocked & (1 << slot):
                print(skip_msg + 'Slot has ben locked')
                continue
            if config.KeyConfig[slot].ReqAuth:
                print(skip_msg + 'Slot requires authorization')
                continue
            if config.KeyConfig[slot].PersistentDisable:
                print(skip_msg + 'Slot requires persistent latch')
                continue
​
        print('    Generating key pair in slot {}'.format(slot))
        public_key = bytearray(64)
        assert ATCA_SUCCESS == atcab_genkey(slot, public_key)
        print(convert_ec_pub_to_pem(public_key))
​
​
if __name__ == '__main__':
    parser = setup_example_runner(__file__)
    parser.add_argument('--i2c', help='I2C Address (in hex)')
    parser.add_argument('--gen', default=True, help='Generate new keys')
    args = parser.parse_args()
​
    if args.i2c is not None:
        args.i2c = int(args.i2c, 16)
​
    print('\nConfiguring the device with an example configuration')
    configure_device(args.iface, args.device, args.i2c, args.gen, **parse_interface_params(args.params))
    print('\nDevice Successfully Configured')