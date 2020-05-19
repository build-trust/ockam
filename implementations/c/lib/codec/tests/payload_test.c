#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "ockam/error.h"
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

#define MAX_DATA_SIZE CODEC_MAX_VLU2_SIZE - sizeof(uint16_t)

#include <stdio.h>
extern void print_uint8_str(uint8_t *p, uint16_t size, char *msg);

uint8_t *test_payload = NULL;
uint8_t *end_payload = NULL;
uint8_t *encoded_payload = NULL;

int _test_codec_payload_setup(void **state) {
  int status = 0;

  test_payload = malloc(CODEC_MAX_VLU2_SIZE);
  if (NULL == test_payload) {
    status = OCKAM_ERROR_INTERFACE_CODEC;
    goto exit_block;
  }

  end_payload = malloc(CODEC_MAX_VLU2_SIZE);
  if (NULL == end_payload) {
    status = OCKAM_ERROR_INTERFACE_CODEC;
    goto exit_block;
  }

  encoded_payload = malloc(0xffffu);
  if (NULL == encoded_payload) {
    status = OCKAM_ERROR_INTERFACE_CODEC;
    goto exit_block;
  }

  for (int i = 0; i < CODEC_MAX_VLU2_SIZE; ++i) {
    test_payload[i] = (uint8_t)i;
  }

exit_block:
  return status;
}

void _test_codec_payload(void **state) {
  uint8_t *out = 0;
  uint8_t *in = 0;
  codec_payload_t payload_in;
  codec_payload_t payload_out;

  payload_in.data = test_payload;
  payload_out.data = end_payload;

  for (unsigned i = 0; i < MAX_DATA_SIZE; ++i) {
    memset(end_payload, 0, CODEC_MAX_VLU2_SIZE);

    payload_in.data_length = i;
    out = encode_payload(encoded_payload, &payload_in);
    if (i >= CODEC_MAX_VLU2_SIZE) {
      assert_null(out);
    } else {
      payload_out.data_length = 0;
      in = decode_payload(encoded_payload, &payload_out);
      assert_int_equal(i, payload_out.data_length);
      assert_int_equal(0, memcmp(payload_in.data, payload_out.data, i));
    }
  }
}

int _test_codec_payload_teardown(void **state) {
  if (0 != test_payload) free(test_payload);
  if (0 != end_payload) free(end_payload);
  if (0 != encoded_payload) free(encoded_payload);

  return 0;
}