#include <stdint.h>
#include <string.h>
#include "ockam/log.h"
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_ockam_wire(uint8_t* p_encoded)
{
  ockam_error_t error = ockam_codec_error_none;
  if (!p_encoded) {
    error.code = OCKAM_CODEC_ERROR_INVALID_PARAM;
    goto exit;
  }
  p_encoded = encode_variable_length_encoded_u2le(p_encoded, OCKAM_WIRE_PROTOCOL_VERSION);
exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    p_encoded = NULL;
  }
  return p_encoded;
}

uint8_t* decode_ockam_wire(uint8_t* p_encoded)
{
  ockam_error_t error   = ockam_codec_error_none;
  uint16_t      version = 0;
  if (!p_encoded) {
    error.code = OCKAM_CODEC_ERROR_INVALID_PARAM;
    goto exit;
  }
  p_encoded = decode_variable_length_encoded_u2le(p_encoded, &version);
  if (OCKAM_WIRE_PROTOCOL_VERSION != version) {
    error.code = OCKAM_CODEC_ERROR_NOT_IMPLEMENTED;
    goto exit;
  }
exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    p_encoded = NULL;
  }
  return p_encoded;
}
