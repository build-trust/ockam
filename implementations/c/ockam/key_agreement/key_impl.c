#include "ockam/error.h"
#include "ockam/log/syslog.h"
#include "ockam/key_agreement.h"
#include "ockam/key_agreement/impl.h"
#include "ockam/memory.h"

ockam_memory_t* gp_ockam_key_memory = NULL;

ockam_error_t ockam_key_initiate(ockam_key_t* p_key)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_key) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }

  error = p_key->dispatch->initiate(p_key->context);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_key_respond(ockam_key_t* p_key)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_key) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }

  error = p_key->dispatch->respond(p_key->context);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_key_encrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_size, size_t* msg_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_key || !payload || !msg || !msg_size) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }
  if (!payload_size || !msg_size) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }

  error = p_key->dispatch->encrypt(p_key->context, payload, payload_size, msg, msg_size, msg_length);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_key_decrypt(
  ockam_key_t* p_key, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* payload_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_key) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }
  if (!payload || !msg || !payload_size || !msg_length || !payload_length) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }

  error = p_key->dispatch->decrypt(p_key->context, payload, payload_size, msg, msg_length, payload_length);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_key_deinit(ockam_key_t* p_key)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_key) {
    error = KEYAGREEMENT_ERROR_PARAMETER;
    goto exit;
  }

  error = p_key->dispatch->deinit(p_key->context);

exit:
  if (error) log_error(error, __func__);
  return error;
}
