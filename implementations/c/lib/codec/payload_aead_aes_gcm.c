#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "codec_local.h"
/**
 * encode_payload_aead_aes_gcm
 * @param encoded [out] - pointer to buffer to receive encoded bytes
 * @param payload [in] - pointer to payload
 * @return - out + number of encoded bytes written
 */
uint8_t *encode_payload_aead_aes_gcm(uint8_t *encoded, PayloadAeadAesGcm *payload) {
  uint16_t encrypted_length = 0;

  if (0 == payload) encoded = 0;
  if (0 == encoded) goto exit_block;

  encrypted_length = payload->length - sizeof(payload->tag) - sizeof(payload->length);

  encoded = encode_variable_length_encoded_u2le(encoded, payload->length);
  if (0 == encoded) goto exit_block;

  memcpy(encoded, payload->encrypted_data, encrypted_length);
  encoded += encrypted_length;

  memcpy(encoded, payload->tag, TAG_SIZE);
  encoded += TAG_SIZE;

exit_block:
  return encoded;
}
/**
 * decode_payload_aead_aes_gcm
 * @param encoded [in] - pointer to encoded bytes
 * @param payload [out] - pointer to payload
 * @return - in + number of bytes decoded
 */
uint8_t *decode_payload_aead_aes_gcm(uint8_t *encoded, PayloadAeadAesGcm *payload) {
  if (0 == payload) encoded = 0;
  if (0 == encoded) goto exit_block;

  encoded = decode_variable_length_encoded_u2le(encoded, &payload->length);
  memcpy(payload->encrypted_data, encoded, payload->length - TAG_SIZE - sizeof(payload->length));
  encoded += payload->length;
  memcpy(payload->tag, encoded, TAG_SIZE);
  encoded += TAG_SIZE;

exit_block:
  return encoded;
}