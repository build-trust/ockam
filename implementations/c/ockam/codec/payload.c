#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "ockam/codec.h"

/**
 * encode_payload
 * @param encoded [out] - buffer for encoded bytes
 * @param length [in] - size of buffer
 * @param data [in] - data to encode
 * @param data_length [in] - bytes to encode
 * @return
 */
uint8_t* encode_payload(uint8_t* encoded, codec_payload_t* kt_payload)
{
  if (0 == encoded) goto exit_block;

  //  encoded = encode_variable_length_encoded_u2le(encoded, kt_payload->data_length);
  //  if (0 == encoded) goto exit_block;
  //
  memcpy(encoded, kt_payload->data, kt_payload->data_length);
  encoded += kt_payload->data_length;

exit_block:
  return encoded;
}

/**
 * decode_payload
 * @param encoded [in] - encoded bytes
 * @param data [out] - decoded bytes
 * @param data_length [in/out] - in: size of data/out: bytes decoded
 * @return
 */
uint8_t* decode_payload(uint8_t* encoded, codec_payload_t* kt_payload)
{
  uint16_t length;

  if (0 == encoded) goto exit_block;

  encoded                 = decode_variable_length_encoded_u2le(encoded, &length);
  kt_payload->data_length = length;

  memcpy(kt_payload->data, encoded, length);
  encoded += length;

exit_block:
  return encoded;
}
