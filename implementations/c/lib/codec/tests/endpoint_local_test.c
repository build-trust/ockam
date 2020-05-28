#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "ockam/error.h"
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

#define ENDPOINT_DATA_SHORT_SIZE 0x007f
#define ENDPOINT_DATA_LONG_SIZE  0x3fff

uint8_t* codec_test_endpoint_encoded = 0;
uint8_t* endpoint_short_in;
uint8_t* endpoint_long_in;

uint8_t* endpoint_short_decoded;
uint8_t* endpoint_long_decoded;

int _test_local_endpoint_setup(void** state)
{
  int status                  = 0;
  codec_test_endpoint_encoded = malloc(0xffff);
  if (0 == codec_test_endpoint_encoded) {
    status = -1;
    goto exit_block;
  }
  endpoint_short_in = malloc(ENDPOINT_DATA_SHORT_SIZE);
  if (0 == endpoint_short_in) {
    status = -1;
    goto exit_block;
  }
  endpoint_long_in = malloc(ENDPOINT_DATA_LONG_SIZE);
  if (0 == endpoint_long_in) {
    status = -1;
    goto exit_block;
  }
  endpoint_short_decoded = malloc(ENDPOINT_DATA_SHORT_SIZE);
  if (0 == endpoint_short_decoded) {
    status = -1;
    goto exit_block;
  }
  endpoint_long_decoded = malloc(ENDPOINT_DATA_LONG_SIZE);
  if (0 == endpoint_long_decoded) {
    status = -1;
    goto exit_block;
  }

exit_block:
  return status;
}

void _test_local_endpoint(void** state)
{
  uint8_t* encoded = codec_test_endpoint_encoded;

  KTLocalEndpoint localEndpointShortIn;
  KTLocalEndpoint localEndpointShortDecoded;
  KTLocalEndpoint localEndpointLongIn;
  KTLocalEndpoint localEndpointLongDecoded;

  CodecEndpointType type;

  for (int i = 0; i < ENDPOINT_DATA_SHORT_SIZE; ++i) endpoint_short_in[i] = i;
  for (int i = 0; i < ENDPOINT_DATA_LONG_SIZE; ++i) endpoint_long_in[i] = i;

  localEndpointShortIn.length = ENDPOINT_DATA_SHORT_SIZE;
  localEndpointShortIn.data   = endpoint_short_in;

  localEndpointShortDecoded.length = 0;
  localEndpointShortDecoded.data   = endpoint_short_decoded;

  encoded = encode_endpoint(encoded, kLocal, (uint8_t*) &localEndpointShortIn);
  assert_ptr_not_equal(encoded, 0);
  encoded = codec_test_endpoint_encoded;
  encoded = decode_endpoint(encoded, &type, (uint8_t*) &localEndpointShortDecoded);
  assert_ptr_not_equal(encoded, 0);
  assert_int_equal(kLocal, type);
  assert_int_equal(localEndpointShortDecoded.length, ENDPOINT_DATA_SHORT_SIZE);
  assert_int_equal(0, memcmp(localEndpointShortDecoded.data, localEndpointShortIn.data, ENDPOINT_DATA_SHORT_SIZE));
}

int _test_local_endpoint_teardown(void** state)
{
  int status = 0;

  if (0 != codec_test_endpoint_encoded) free(codec_test_endpoint_encoded);
  if (0 != endpoint_short_in) free(endpoint_short_in);
  if (0 != endpoint_long_in) free(endpoint_long_in);
  if (0 != endpoint_short_decoded) free(endpoint_short_decoded);
  if (0 != endpoint_long_decoded) free(endpoint_long_decoded);

exit_block:
  return status;
}
