#include <stdint.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_ockam_wire(uint8_t* p_encoded)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  if (!p_encoded) {
    error = CODEC_ERROR_PARAMETER;
    goto exit;
  }
  p_encoded = encode_variable_length_encoded_u2le(p_encoded, OCKAM_WIRE_PROTOCOL_VERSION);
exit:
  if (error) {
    log_error(error, __func__);
    p_encoded = NULL;
  }
  return p_encoded;
}

uint8_t* decode_ockam_wire(uint8_t* p_encoded, uint16_t* p_version)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  if (!p_encoded) {
    error = CODEC_ERROR_PARAMETER;
    goto exit;
  }
  p_encoded = decode_variable_length_encoded_u2le(p_encoded, p_version);
exit:
  if (error) {
    log_error(error, __func__);
    p_encoded = NULL;
  }
  return p_encoded;
}
