#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_public_key(uint8_t* encoded, codec_public_key_t* public_key)
{
  *encoded++ = (uint8_t) public_key->type;

  memcpy(encoded, public_key->x, KEY_CURVE_SIZE);
  encoded += KEY_CURVE_SIZE;

  if (kCurveP256Uncompressed == public_key->type) {
    memcpy(encoded, public_key->y, KEY_CURVE_SIZE);
    encoded += KEY_CURVE_SIZE;
  }

  return encoded;
}

uint8_t* decode_public_key(uint8_t* encoded, codec_public_key_t* public_key)
{
  public_key->type = *encoded++;

  memcpy(public_key->x, encoded, KEY_CURVE_SIZE);
  encoded += KEY_CURVE_SIZE;

  if (kCurveP256Uncompressed == public_key->type) {
    memcpy(public_key->y, encoded, KEY_CURVE_SIZE);
    encoded += KEY_CURVE_SIZE;
  }

  return encoded;
}
