#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "ockam/error.h"
#include "cmocka.h"
#include "codec_tests.h"
#include "../codec_local.h"

#include <stdio.h>
void print_uint8_str(uint8_t *p, uint16_t size, char *msg) {
  printf("\n%s %d bytes: \n", msg, size);
  for (int i = 0; i < size; ++i) printf("%0.2x", *p++);
  printf("\n");
}

#define MAX_PACKET_SIZE 0x7fffu
#define MAX_ENCRYPTED_SIZE MAX_PACKET_SIZE - TAG_SIZE - sizeof(uint16_t)

PayloadAeadAesGcm *test_payload = NULL;
PayloadAeadAesGcm *end_payload = NULL;
uint8_t *encoded_payload = NULL;

int _test_codec_payload_aead_aes_gcm_setup(void **state) {
  int status = 0;

  test_payload = malloc(0x7fff);
  if (NULL == test_payload) {
    status = kOckamError;
    goto exit_block;
  }

  end_payload = malloc(0x7fff);
  if (NULL == end_payload) {
    status = kOckamError;
    goto exit_block;
  }

  encoded_payload = malloc(0x7fff);
  if (NULL == encoded_payload) {
    status = kOckamError;
    goto exit_block;
  }

  for (int i = 0; i < MAX_ENCRYPTED_SIZE; ++i) {
    test_payload->encrypted_data[i] = i;
  }

exit_block:
  return status;
}

void _test_codec_payload_aead_aes_gcm(void **state) {
  uint8_t *out = NULL;
  uint8_t *_out = NULL;
  uint8_t *in = NULL;

  for (uint16_t i = TAG_SIZE + sizeof(uint16_t); i < MAX_ENCRYPTED_SIZE; ++i) {
    memset(end_payload, 0, MAX_PACKET_SIZE);
    test_payload->length = i;

    out = encode_payload_aead_aes_gcm(encoded_payload, test_payload);
    if (i & 0x8000u) {
      assert_null(out);
    } else {
      in = decode_payload_aead_aes_gcm(encoded_payload, end_payload);
      assert_int_equal(0, memcmp(test_payload, end_payload, i));
    }
  }
}

int _test_codec_payload_aead_aes_gcm_teardown(void **state) {
  if (0 != test_payload) free(test_payload);
  if (0 != end_payload) free(end_payload);
  if (0 != encoded_payload) free(encoded_payload);

  return 0;
}