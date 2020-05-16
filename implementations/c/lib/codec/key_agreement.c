#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t *encode_key_agreement(uint8_t *encoded, KTPayload *kt_payload) {
  return encode_payload(encoded, kt_payload);
}

uint8_t *decode_key_agreement(uint8_t *encoded, KTPayload *kt_payload) {
  return decode_payload(encoded, kt_payload);
}
