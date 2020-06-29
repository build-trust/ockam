#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include <stdio.h>
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

uint8_t ipv4[4]  = { 127, 0, 0, 1 };
uint8_t ipv6[16] = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 };

void _test_route()
{
  codec_address_t addresses[5];
  codec_address_t decoded_address;
  uint8_t         encoded[1024];
  uint8_t*        p_encoded = encoded;

  memset(encoded, 0, sizeof(encoded));

  addresses[0].type                                  = ADDRESS_TCP;
  addresses[0].address.tcp_address.host_address.type = HOST_ADDRESS_IPV4;
  memcpy(addresses[0].address.tcp_address.host_address.ip_address.ipv4, ipv4, 4);
  addresses[0].address.tcp_address.port = 8000;

  addresses[1].type                                  = ADDRESS_TCP;
  addresses[1].address.tcp_address.host_address.type = HOST_ADDRESS_IPV6;
  memcpy(addresses[1].address.tcp_address.host_address.ip_address.ipv6, ipv6, 16);
  addresses[1].address.tcp_address.port = 8000;

  addresses[2].type                                  = ADDRESS_UDP;
  addresses[2].address.udp_address.host_address.type = HOST_ADDRESS_IPV4;
  memcpy(addresses[2].address.udp_address.host_address.ip_address.ipv4, ipv4, 4);
  addresses[2].address.udp_address.port = 6000;

  addresses[3].type                                  = ADDRESS_UDP;
  addresses[3].address.udp_address.host_address.type = HOST_ADDRESS_IPV6;
  memcpy(addresses[3].address.udp_address.host_address.ip_address.ipv6, ipv6, 16);
  addresses[3].address.udp_address.port = 6000;

  addresses[4].type = ADDRESS_LOCAL;
  memcpy(addresses[4].address.local_address, "01234567", 8);

  p_encoded = encode_address(p_encoded, &addresses[0]);
  p_encoded = encoded;
  memset(&decoded_address, 0, sizeof(decoded_address));
  p_encoded = decode_address(p_encoded, &decoded_address);
  assert_non_null(p_encoded);

  assert_int_equal(decoded_address.type, ADDRESS_TCP);
  assert_int_equal(decoded_address.address.tcp_address.host_address.type, HOST_ADDRESS_IPV4);
  assert_int_equal(memcmp(decoded_address.address.tcp_address.host_address.ip_address.ipv4, ipv4, 4), 0);
  assert_int_equal(decoded_address.address.tcp_address.port, 8000);

  p_encoded = encoded;
  p_encoded = encode_address(p_encoded, &addresses[1]);
  p_encoded = encoded;
  memset(&decoded_address, 0, sizeof(decoded_address));
  p_encoded = decode_address(p_encoded, &decoded_address);
  assert_non_null(p_encoded);

  assert_int_equal(decoded_address.type, ADDRESS_TCP);
  assert_int_equal(decoded_address.address.tcp_address.host_address.type, HOST_ADDRESS_IPV6);
  assert_int_equal(memcmp(decoded_address.address.tcp_address.host_address.ip_address.ipv6, ipv6, 16), 0);
  assert_int_equal(decoded_address.address.tcp_address.port, 8000);

  p_encoded = encoded;
  p_encoded = encode_address(p_encoded, &addresses[2]);
  p_encoded = encoded;
  memset(&decoded_address, 0, sizeof(decoded_address));
  p_encoded = decode_address(p_encoded, &decoded_address);
  assert_non_null(p_encoded);

  assert_int_equal(decoded_address.type, ADDRESS_UDP);
  assert_int_equal(decoded_address.address.udp_address.host_address.type, HOST_ADDRESS_IPV4);
  assert_int_equal(memcmp(decoded_address.address.udp_address.host_address.ip_address.ipv4, ipv4, 4), 0);
  assert_int_equal(decoded_address.address.udp_address.port, 6000);

  p_encoded = encoded;
  p_encoded = encode_address(p_encoded, &addresses[3]);
  p_encoded = encoded;
  memset(&decoded_address, 0, sizeof(decoded_address));
  p_encoded = decode_address(p_encoded, &decoded_address);
  assert_non_null(p_encoded);

  assert_int_equal(decoded_address.type, ADDRESS_UDP);
  assert_int_equal(decoded_address.address.udp_address.host_address.type, HOST_ADDRESS_IPV6);
  assert_int_equal(memcmp(decoded_address.address.udp_address.host_address.ip_address.ipv6, ipv6, 16), 0);
  assert_int_equal(decoded_address.address.udp_address.port, 6000);

  p_encoded = encoded;
  p_encoded = encode_address(p_encoded, &addresses[4]);
  p_encoded = encoded;
  memset(&decoded_address, 0, sizeof(decoded_address));
  p_encoded = decode_address(p_encoded, &decoded_address);
  assert_non_null(p_encoded);
  assert_int_equal(memcmp(addresses[4].address.local_address, "01234567", 8), 0);
  assert_int_equal(addresses[4].type, ADDRESS_LOCAL);
}