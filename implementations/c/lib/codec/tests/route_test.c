#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

uint8_t ipv4[4] = {127, 0, 0, 1};
uint8_t ipv6[16] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16};




void _test_route()
{
  codec_route_t   route;
  codec_address_t addresses[4];
  uint8_t encoded[1024];
  uint8_t* p_encoded = encoded;

  memset(encoded, 0, sizeof(encoded));

  addresses[0].type = ADDRESS_TCP;
  addresses[0].socket_address.tcp_address.host_address.type = HOST_ADDRESS_IPV4;
  memcpy(addresses[0].socket_address.tcp_address.host_address.ip_address.ipv4, ipv4, 4);
  addresses[0].socket_address.tcp_address.port = 8000;

  addresses[1].type = ADDRESS_TCP;
  addresses[1].socket_address.tcp_address.host_address.type = HOST_ADDRESS_IPV6;
  memcpy(addresses[1].socket_address.tcp_address.host_address.ip_address.ipv6, ipv6, 16);
  addresses[1].socket_address.tcp_address.port = 8000;

  addresses[2].type = ADDRESS_UDP;
  addresses[2].socket_address.udp_address.host_address.type = HOST_ADDRESS_IPV4;
  memcpy(addresses[2].socket_address.udp_address.host_address.ip_address.ipv4, ipv4, 4);
  addresses[2].socket_address.udp_address.port = 6000;

  addresses[3].type = ADDRESS_UDP;
  addresses[3].socket_address.udp_address.host_address.type = HOST_ADDRESS_IPV6;
  memcpy(addresses[3].socket_address.udp_address.host_address.ip_address.ipv6, ipv6, 16);
  addresses[3].socket_address.udp_address.port = 6000;

  route.count_addresses = 4;
  route.p_addresses = addresses;

  p_encoded = encode_route(p_encoded, &route);
  assert_non_null(p_encoded);

  p_encoded = encoded;
  memset(&addresses, 0, sizeof(addresses));
  memset(&route, 0, sizeof(route));
  route.p_addresses = addresses;

  p_encoded = decode_route(p_encoded, &route);
  assert_non_null(p_encoded);

  assert_int_equal(route.count_addresses, 4);
  assert_int_equal(addresses[0].type, ADDRESS_TCP);
  assert_int_equal(addresses[0].socket_address.tcp_address.host_address.type, HOST_ADDRESS_IPV4);
  assert_int_equal(memcmp(addresses[0].socket_address.tcp_address.host_address.ip_address.ipv4, ipv4, 4), 0);
  assert_int_equal(addresses[0].socket_address.tcp_address.port, 8000);

  assert_int_equal(addresses[1].type, ADDRESS_TCP);
  assert_int_equal(addresses[1].socket_address.tcp_address.host_address.type, HOST_ADDRESS_IPV6);
  assert_int_equal(memcmp(addresses[1].socket_address.tcp_address.host_address.ip_address.ipv6, ipv6, 16), 0);
  assert_int_equal(addresses[1].socket_address.tcp_address.port, 8000);

  assert_int_equal(addresses[2].type, ADDRESS_UDP);
  assert_int_equal(addresses[2].socket_address.udp_address.host_address.type, HOST_ADDRESS_IPV4);
  assert_int_equal(memcmp(addresses[2].socket_address.udp_address.host_address.ip_address.ipv4, ipv4, 4), 0);
  assert_int_equal(addresses[2].socket_address.udp_address.port, 6000);

  assert_int_equal(addresses[3].type, ADDRESS_UDP);
  assert_int_equal(addresses[3].socket_address.udp_address.host_address.type, HOST_ADDRESS_IPV6);
  assert_int_equal(memcmp(addresses[3].socket_address.udp_address.host_address.ip_address.ipv6, ipv6, 16), 0);
  assert_int_equal(addresses[3].socket_address.udp_address.port, 6000);

}