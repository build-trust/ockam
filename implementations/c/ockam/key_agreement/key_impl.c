#include "ockam/error.h"
#include "ockam/log.h"
#include "ockam/key_agreement.h"
#include "ockam/key_agreement/impl.h"
#include "ockam/memory.h"

const char* const OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_DOMAIN = "OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_DOMAIN";

static const ockam_error_t ockam_key_agreement_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_DOMAIN
};

ockam_memory_t* gp_ockam_key_memory = NULL;

ockam_error_t ockam_key_m1_make(ockam_key_t* p_key, uint8_t* m1, size_t m1_size, size_t* m1_length)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m1_make(p_key->context, m1, m1_size, m1_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_m2_make(ockam_key_t* p_key, uint8_t* m2, size_t m2_size, size_t* m2_length)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m2_make(p_key->context, m2, m2_size, m2_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_m3_make(ockam_key_t* p_key, uint8_t* m3, size_t m3_size, size_t* m1_length)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m3_make(p_key->context, m3, m3_size, m1_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_m1_process(ockam_key_t* p_key, uint8_t* m1)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m1_process(p_key->context, m1);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_m2_process(ockam_key_t* p_key, uint8_t* m2)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m2_process(p_key->context, m2);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_m3_process(ockam_key_t* p_key, uint8_t* m3)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->m3_process(p_key->context, m3);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_initiator_epilogue(ockam_key_t* key)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = key->dispatch->initiator_epilogue(key);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_responder_epilogue(ockam_key_t* key)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = key->dispatch->responder_epilogue(key);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_encrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_size, size_t* msg_length)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key || !payload || !msg || !msg_length) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }
  if (!payload_size || !msg_length) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->encrypt(p_key->context, payload, payload_size, msg, msg_size, msg_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_decrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* payload_length)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }
  if (!payload || !msg || !payload_size || !msg_length || !payload_length) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->decrypt(p_key->context, payload, payload_size, msg, msg_length, payload_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t ockam_key_deinit(ockam_key_t* p_key)
{
  ockam_error_t error = ockam_key_agreement_interface_error_none;

  if (!p_key) {
    error.code = OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = p_key->dispatch->deinit(p_key->context);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}
