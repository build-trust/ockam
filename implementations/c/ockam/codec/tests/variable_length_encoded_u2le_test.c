#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "codec_tests.h"
#include "cmocka.h"
#include "ockam/codec.h"

#include <stdio.h>

#define TEST_SET_SIZE 0xffffu

uint16_t* test_nums;
uint8_t*  ul2_encoded;

int _test_codec_variable_length_encoded_u2le_setup(void** state)
{
  int status = 0;

  test_nums = (uint16_t*) malloc(TEST_SET_SIZE * sizeof(uint16_t));
  if (0 == test_nums) {
    status = -1;
    goto exit_block;
  }

  ul2_encoded = (uint8_t*) malloc(TEST_SET_SIZE * sizeof(uint8_t));
  if (0 == ul2_encoded) {
    status = -1;
    goto exit_block;
  }

  for (int i = 0; i <= TEST_SET_SIZE; ++i) { test_nums[i] = i; }

exit_block:
  return status;
}

void _test_codec_variable_length_encoded_u2le(void** state)
{
  uint8_t* out                = ul2_encoded;
  uint8_t* _out               = 0;
  uint8_t* in                 = ul2_encoded;
  uint8_t* in_end             = 0;
  uint16_t value              = 0;
  uint16_t ul2_encoded_length = 0;
  int      i                  = 0;

  for (i = 0; i <= TEST_SET_SIZE; ++i) {
    _out = out;
    out  = encode_variable_length_encoded_u2le(out, test_nums[i]);
    if (test_nums[i] & 0xC000u) {
      assert_null(out);
      out = _out;
    }
  }

  ul2_encoded_length = out - ul2_encoded;
  in_end             = in + ul2_encoded_length;

  i = 0;
  while (in < in_end) {
    in = decode_variable_length_encoded_u2le(in, &value);
    assert_int_equal(value, test_nums[i]);
    assert_int_equal(0, (value & 0x8000u));
    i++;
  }
}

int _test_codec_variable_length_encoded_u2le_teardown(void** state)
{
  if (0 != ul2_encoded) free(ul2_encoded);
  if (0 != test_nums) free(test_nums);

  return 0;
}
