#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "ockam/codec.h"
/**
 * encode_payload_aead_aes_gcm
 * @param encoded [out] - pointer to buffer to receive encoded bytes
 * @param payload [in] - pointer to payload
 * @return - out + number of encoded bytes written
 */
uint8_t* encode_payload_aead_aes_gcm(uint8_t* encoded, codec_aead_aes_gcm_payload_t* payload)
{
  uint16_t encoded_length = 0;

  if (0 == payload) encoded = 0;
  if (0 == encoded) goto exit_block;

  encoded_length = payload->encrypted_data_length + AEAD_AES_GCM_TAG_SIZE;

  encoded = encode_variable_length_encoded_u2le(encoded, encoded_length);
  if (0 == encoded) goto exit_block;

  memcpy(encoded, payload->encrypted_data, payload->encrypted_data_length);
  encoded += payload->encrypted_data_length;

  memcpy(encoded, payload->tag, AEAD_AES_GCM_TAG_SIZE);
  encoded += AEAD_AES_GCM_TAG_SIZE;

exit_block:
  return encoded;
}
/**
 * decode_payload_aead_aes_gcm
 * @param encoded [in] - pointer to encoded bytes
 * @param payload [out] - pointer to payload
 * @return - in + number of bytes decoded
 */
uint8_t* decode_payload_aead_aes_gcm(uint8_t* encoded, codec_aead_aes_gcm_payload_t* payload)
{
  uint16_t decoded_length = 0;

  if (0 == payload) encoded = 0;
  if (0 == encoded) goto exit_block;

  encoded                        = decode_variable_length_encoded_u2le(encoded, &decoded_length);
  payload->encrypted_data_length = decoded_length - AEAD_AES_GCM_TAG_SIZE;
  memcpy(payload->encrypted_data, encoded, payload->encrypted_data_length);
  encoded += payload->encrypted_data_length;
  memcpy(payload->tag, encoded, AEAD_AES_GCM_TAG_SIZE);
  encoded += AEAD_AES_GCM_TAG_SIZE;

exit_block:
  return encoded;
}
