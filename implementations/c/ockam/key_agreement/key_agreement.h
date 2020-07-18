#ifndef OCKAM_HANDSHAKE_H
#define OCKAM_HANDSHAKE_H

#include <stdlib.h>
#include <stdint.h>

#include "ockam/error.h"

#define KEYAGREEMENT_ERROR_TEST      (OCKAM_ERROR_INTERFACE_KEYAGREEMENT | 1u)
#define KEYAGREEMENT_ERROR_FAIL      (OCKAM_ERROR_INTERFACE_KEYAGREEMENT | 2U)
#define KEYAGREEMENT_ERROR_PARAMETER (OCKAM_ERROR_INTERFACE_KEYAGREEMENT | 3u)

typedef struct ockam_key ockam_key_t;

ockam_error_t ockam_key_initiate(ockam_key_t* p_key);

ockam_error_t ockam_key_respond(ockam_key_t* p_key);

ockam_error_t ockam_key_encrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* msg_size);

ockam_error_t ockam_key_decrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* payload_length);

ockam_error_t ockam_key_deinit(ockam_key_t*);

#endif
