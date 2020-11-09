""" Common helper functions for cryptoauthlib examples """
import argparse
import os
import base64
import sys

# Maps common name to the specific name used internally
atca_names_map = {'i2c': 'i2c', 'hid': 'kithid', 'sha': 'sha20x', 'ecc': 'eccx08'}

try:
    FileNotFoundError
except NameError:
    FileNotFoundError = IOError


def get_device_name(revision):
    """
    Returns the device name based on the info byte array values returned by atcab_info
    """
    devices = {0x10: 'ATECC108A', 
               0x50: 'ATECC508A', 
               0x60: 'ATECC608A',
               0x00: 'ATSHA204A',
               0x02: 'ATSHA204A'}
    return devices.get(revision[2], 'UNKNOWN')


def get_device_type_id(name):
    """
    Returns the ATCADeviceType value based on the device name
    """
    devices = {'ATSHA204A': 0,
               'ATECC108A': 1, 
               'ATECC508A': 2,
               'ATECC608A': 3,
               'UNKNOWN': 0x20 }
    return devices.get(name.upper())


def setup_example_runner(module):
    """
    Common helper function that sets up the script entry for all examples
    """
    example = os.path.basename(module).split('.')[0]

    try:
        with open(example + '.md', 'r') as f:
            details = f.read()
    except FileNotFoundError:
        details = example.upper() + ' Example'

    parser = argparse.ArgumentParser(description=details, 
                                 formatter_class=argparse.RawDescriptionHelpFormatter)

    parser.add_argument('-i', '--iface', default='hid', choices=['i2c', 'hid'], help='Interface type (default: hid)')
    parser.add_argument('-d', '--device', default='ecc', choices=['ecc', 'sha'], help='Device type (default: ecc)')
    parser.add_argument('-p', '--params', nargs='*', help='Interface Parameters in the form key=value')

    return parser


def parse_interface_params(list):
    """
    Parse a variable list of key=value args into a dictionary suitable for kwarg usage
    """
    return {} if list is None else dict([s.split('=') for s in list])


def pretty_print_hex(a, l=16, indent=''):
    """
    Format a list/bytes/bytearray object into a formatted ascii hex string
    """
    lines = []
    a = bytearray(a)
    for x in range(0, len(a), l):
        lines.append(indent + ' '.join(['{:02X}'.format(y) for y in a[x:x+l]]))
    return '\n'.join(lines)


def convert_ec_pub_to_pem(raw_pub_key):
    """
    Convert to the key to PEM format. Expects bytes
    """
    public_key_der = bytearray.fromhex('3059301306072A8648CE3D020106082A8648CE3D03010703420004') + raw_pub_key
    public_key_b64 = base64.b64encode(public_key_der).decode('ascii')
    public_key_pem = (
        '-----BEGIN PUBLIC KEY-----\n'
        + '\n'.join(public_key_b64[i:i + 64] for i in range(0, len(public_key_b64), 64)) + '\n'
        + '-----END PUBLIC KEY-----'
    )
    return public_key_pem


def check_if_rpi():
    """
    Does a basic check to see if the script is running on a Raspberry Pi
    """
    is_rpi = False
    try:
        with open('/sys/firmware/devicetree/base/model', 'r') as f:
            if f.readline().startswith('Raspberry'):
                is_rpi = True
    except FileNotFoundError:
        is_rpi = False

    return is_rpi
