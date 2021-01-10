#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "ockam/error.h"
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

#include <stdio.h>
extern void print_uint8_str(uint8_t* p, uint16_t size, char* msg);

#define MAX_ENCRYPTED_SIZE CODEC_MAX_VLU2_SIZE - AEAD_AES_GCM_TAG_SIZE - sizeof(uint16_t)

uint8_t* aag_test_payload   = 0;
uint8_t* aag_end_payload    = 0;
uint8_t* aag_encoded_stream = 0;

int _test_codec_payload_aead_aes_gcm_setup(void** state)
{
  int status = 0;

  aag_test_payload = malloc(CODEC_MAX_VLU2_SIZE);
  if (NULL == aag_test_payload) {
    status = -1;
    goto exit_block;
  }

  aag_end_payload = malloc(CODEC_MAX_VLU2_SIZE);
  if (NULL == aag_end_payload) {
    status = -1;
    goto exit_block;
  }

  aag_encoded_stream = malloc(CODEC_MAX_VLU2_SIZE);
  if (NULL == aag_encoded_stream) {
    status = -1;
    goto exit_block;
  }

  for (int i = 0; i < MAX_ENCRYPTED_SIZE; ++i) { aag_test_payload[i] = i; }

exit_block:
  return status;
}

void _test_codec_payload_aead_aes_gcm(void** state)
{
  codec_aead_aes_gcm_payload_t aag_in;
  codec_aead_aes_gcm_payload_t aag_out;
  uint8_t*                     out  = NULL;
  uint8_t*                     _out = NULL;
  uint8_t*                     in   = NULL;

  aag_in.encrypted_data      = aag_test_payload;
  aag_in.encrypted_data_size = MAX_ENCRYPTED_SIZE;
  for (int i = 0; i < AEAD_AES_GCM_TAG_SIZE; ++i) aag_in.tag[i] = i;
  aag_out.encrypted_data      = aag_end_payload;
  aag_out.encrypted_data_size = MAX_ENCRYPTED_SIZE;

  for (uint16_t i = 0; i < MAX_ENCRYPTED_SIZE; ++i) {
    memset(aag_out.encrypted_data, 0, MAX_ENCRYPTED_SIZE);
    memset(aag_out.tag, 0, AEAD_AES_GCM_TAG_SIZE);
    aag_out.encrypted_data_length = 0;

    aag_in.encrypted_data_length = i;

    out = encode_payload_aead_aes_gcm(aag_encoded_stream, &aag_in);
    if (i & 0x8000u) {
      assert_null(out);
    } else {
      decode_payload_aead_aes_gcm(aag_encoded_stream, &aag_out);
      assert_int_equal(0, memcmp(aag_in.encrypted_data, aag_out.encrypted_data, i));
      assert_int_equal(0, memcmp(aag_in.tag, aag_out.tag, AEAD_AES_GCM_TAG_SIZE));
      assert_int_equal(i, aag_out.encrypted_data_length);
    }
  }
}

int _test_codec_payload_aead_aes_gcm_teardown(void** state)
{
  if (0 != aag_test_payload) free(aag_test_payload);
  if (0 != aag_end_payload) free(aag_end_payload);
  if (0 != aag_encoded_stream) free(aag_encoded_stream);

  return 0;
}
