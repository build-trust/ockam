#include <stdint.h>
#include "ockam/error.h"

#define TAG_SIZE 16

typedef enum {
  kInvalidParams = kOckamErrorCodec | 0x0001u,
} OckamCodecError;

typedef struct {
  uint16_t length;
  uint8_t tag[TAG_SIZE];
  uint8_t encrypted_data[];
} PayloadAeadAesGcm;

uint8_t *decode_variable_length_encoded_u2le(uint8_t *in, uint16_t *val);
uint8_t *encode_variable_length_encoded_u2le(uint8_t *out, uint16_t val);
uint8_t *encode_payload_aead_aes_gcm(uint8_t *out, PayloadAeadAesGcm *payload);
uint8_t *decode_payload_aead_aes_gcm(uint8_t *in, PayloadAeadAesGcm *payload);
