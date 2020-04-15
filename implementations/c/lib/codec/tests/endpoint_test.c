#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

uint8_t* encoded_buffer;

int _test_endpoints_setup(void** state) {
  int status = 0;

  encoded_buffer = malloc(0xffff);
  if (0 == encoded_buffer) {
    status = -1;
    goto exit_block;
  }

exit_block:
  return 0;
}

void _test_endpoints(void** state) {
  KTTcpIpv4Endpoint ep_ipv4_in = {{127, 0, 0, 1}, 4000};
  KTTcpIpv4Endpoint ep_ipv4_out;

  CodecEndpointType type;

  uint8_t* encoded = encoded_buffer;

  // IPV4
  encoded = encode_endpoint(encoded, kTcpIpv4, (uint8_t*)&ep_ipv4_in);
  assert_ptr_not_equal(0, encoded);
  encoded = encoded_buffer;
  encoded = decode_endpoint(encoded, &type, (uint8_t*)&ep_ipv4_out);
  assert_ptr_not_equal(0, encoded);
  assert_int_equal(type, kTcpIpv4);
  assert_int_equal(0, memcmp(&ep_ipv4_in, &ep_ipv4_out, sizeof(KTTcpIpv4Endpoint)));
}

int _test_endpoints_teardown(void** state) {
  int status = 0;

  if (0 != encoded_buffer) free(encoded_buffer);
  return 0;
}
