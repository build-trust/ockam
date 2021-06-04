#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"
#include "ockam/log.h"

uint8_t* address_buffer;

int _test_endpoints_setup(void** state)
{
  int error = 0;

  address_buffer = malloc(0xffff);
  if (0 == address_buffer) {
    error = -1;
    goto exit_block;
  }

exit_block:
  if (error) ockam_log_error("%d", error);
  return 0;
}

void _test_endpoints(void** state)
{
  KTTcpIpv4Endpoint ep_ipv4_in = { { 127, 0, 0, 1 }, 4000 };
  KTTcpIpv4Endpoint ep_ipv4_out;

  CodecEndpointType type;

  uint8_t* encoded = address_buffer;

  // IPV4
  encoded = encode_endpoint(encoded, kTcpIpv4, (uint8_t*) &ep_ipv4_in);
  assert_ptr_not_equal(0, encoded);
  encoded = address_buffer;
  encoded = decode_endpoint(encoded, &type, (uint8_t*) &ep_ipv4_out);
  assert_ptr_not_equal(0, encoded);
  assert_int_equal(type, kTcpIpv4);
  assert_int_equal(0, memcmp(&ep_ipv4_in, &ep_ipv4_out, sizeof(KTTcpIpv4Endpoint)));
}

int _test_endpoints_teardown(void** state)
{
  int error = 0;

  if (0 != address_buffer) free(address_buffer);
  return 0;
}
