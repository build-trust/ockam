#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"
#include "ockam/log.h"

uint8_t* local_data_in;
uint8_t* local_data_out;
uint8_t* encoded_buffer;

int _test_channel_endpoint_setup(void** state)
{
  int error = 0;

  local_data_in = malloc(CODEC_MAX_VLU2_SIZE);
  if (0 == local_data_in) {
    error = -1;
    goto exit_block;
  }
  local_data_out = malloc(CODEC_MAX_VLU2_SIZE);
  if (0 == local_data_out) {
    error = -1;
    goto exit_block;
  }
  encoded_buffer = malloc(0xffff);
  if (0 == encoded_buffer) {
    error = -1;
    goto exit_block;
  }

exit_block:
  if (error) ockam_log_error("%d", error);
  return 0;
}

void _test_channel_endpoint()
{
  KTLocalEndpoint ep_local_in;
  KTLocalEndpoint ep_local_out;

  KTChannelEndpoint ep_channel_in;
  KTChannelEndpoint ep_channel_out;

  CodecEndpointType type;

  uint8_t* encoded = encoded_buffer;

  // Initialize the local endpoints
  ep_local_in.length  = CODEC_MAX_VLU2_SIZE;
  ep_local_in.data    = local_data_in;
  ep_local_out.length = 0;
  ep_local_out.data   = local_data_out;
  memset(local_data_out, 0, CODEC_MAX_VLU2_SIZE);
  memset(local_data_in, 0, CODEC_MAX_VLU2_SIZE);
  ep_local_in.length = CODEC_MAX_VLU2_SIZE;
  for (int i = 0; i < CODEC_MAX_VLU2_SIZE; ++i) ep_local_in.data[i] = i;

  // Initialize the channel endpoints
  memset(&ep_channel_out.public_key, 0, KEY_CURVE_SIZE);
  ep_channel_in.public_key.type = kCurveP256Uncompressed;
  for (int i = 0; i < KEY_CURVE_SIZE; ++i) {
    ep_channel_in.public_key.x[i] = i;
    ep_channel_in.public_key.y[i] = i;
  }

  // Encode
  encoded = encode_endpoint(encoded, kChannel, (uint8_t*) &ep_channel_in);
  assert_ptr_not_equal(encoded, 0);
  encoded = encode_endpoint(encoded, kLocal, (uint8_t*) &ep_local_in);
  assert_ptr_not_equal(encoded, 0);

  // Decode
  encoded = encoded_buffer;
  encoded = decode_endpoint(encoded, &type, (uint8_t*) &ep_channel_out);
  assert_ptr_not_equal(encoded, 0);
  assert_int_equal(type, kChannel);
  assert_int_equal(0, memcmp(&ep_channel_in, &ep_channel_out, sizeof(KTChannelEndpoint)));
  encoded = decode_endpoint(encoded, &type, (uint8_t*) &ep_local_out);
  assert_ptr_not_equal(encoded, 0);
  assert_int_equal(type, kLocal);
  assert_int_equal(memcmp(ep_local_in.data, ep_local_out.data, CODEC_MAX_VLU2_SIZE), 0);
}

int _test_channel_endpoint_teardown(void** state)
{
  int error = 0;

  if (0 != local_data_in) free(local_data_in);
  if (0 != local_data_out) free(local_data_out);
  if (0 != encoded_buffer) free(encoded_buffer);
  return 0;
}
