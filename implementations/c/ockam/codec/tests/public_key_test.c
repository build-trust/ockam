#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <setjmp.h>
#include "ockam/error.h"
#include "cmocka.h"
#include "codec_tests.h"
#include "ockam/codec.h"

void _test_public_key(void** state)
{
  codec_public_key_t pk_in;
  codec_public_key_t pk_out;
  uint8_t            encoded[2 * KEY_CURVE_SIZE + 1];
  uint8_t*           encoded_ptr = NULL;

  memset(pk_in.x, 'O', KEY_CURVE_SIZE);
  memset(pk_in.y, 'K', KEY_CURVE_SIZE);

  memset(&pk_out, 0, sizeof(pk_out));
  pk_in.type  = kCurve25519;
  encoded_ptr = encode_public_key(encoded, &pk_in);
  assert_ptr_equal(encoded_ptr, encoded + KEY_CURVE_SIZE + 1);
  encoded_ptr = decode_public_key(encoded, &pk_out);
  assert_ptr_equal(encoded_ptr, encoded + KEY_CURVE_SIZE + 1);
  assert_int_equal(pk_out.type, kCurve25519);
  assert_int_equal(0, memcmp(pk_in.x, pk_out.x, KEY_CURVE_SIZE));

  memset(&pk_out, 0, sizeof(pk_out));
  pk_in.type  = kCurveP256Uncompressed;
  encoded_ptr = encode_public_key(encoded, &pk_in);
  assert_ptr_equal(encoded_ptr, encoded + (2 * KEY_CURVE_SIZE) + 1);
  encoded_ptr = decode_public_key(encoded, &pk_out);
  assert_ptr_equal(encoded_ptr, encoded + (2 * KEY_CURVE_SIZE) + 1);
  assert_int_equal(pk_out.type, kCurveP256Uncompressed);
  assert_int_equal(0, memcmp(pk_in.x, pk_out.x, KEY_CURVE_SIZE));
  assert_int_equal(0, memcmp(pk_in.y, pk_out.y, KEY_CURVE_SIZE));
}
