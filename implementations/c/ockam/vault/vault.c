#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/vault/impl.h"

const char* const OCKAM_VAULT_INTERFACE_ERROR_DOMAIN = "OCKAM_VAULT_INTERFACE_ERROR_DOMAIN";

static const ockam_error_t ockam_vault_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_VAULT_INTERFACE_ERROR_DOMAIN
};

ockam_error_t ockam_vault_deinit(ockam_vault_t* vault)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->deinit(vault);

exit:
  return error;
}

ockam_error_t ockam_vault_random_bytes_generate(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if ((vault == 0) || buffer == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->random(vault, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t ockam_vault_sha256(ockam_vault_t* vault,
                                 const uint8_t* input,
                                 size_t         input_length,
                                 uint8_t*       digest,
                                 size_t         digest_size,
                                 size_t*        digest_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if ((vault == 0) || (digest == 0)) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->sha256(vault, input, input_length, digest, digest_size, digest_length);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_generate(ockam_vault_t*                         vault,
                                          ockam_vault_secret_t*                  secret,
                                          const ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_generate(vault, secret, attributes);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_import(ockam_vault_t*                         vault,
                                        ockam_vault_secret_t*                  secret,
                                        const ockam_vault_secret_attributes_t* attributes,
                                        const uint8_t*                         input,
                                        size_t                                 input_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_import(vault, secret, attributes, input, input_length);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_export(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* secret,
                                        uint8_t*              output_buffer,
                                        size_t                output_buffer_size,
                                        size_t*               output_buffer_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_export(vault, secret, output_buffer, output_buffer_size, output_buffer_length);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_publickey_get(ockam_vault_t*        vault,
                                               ockam_vault_secret_t* secret,
                                               uint8_t*              output_buffer,
                                               size_t                output_buffer_size,
                                               size_t*               output_buffer_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_publickey_get(vault, secret, output_buffer, output_buffer_size, output_buffer_length);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_attributes_get(ockam_vault_t*                   vault,
                                                ockam_vault_secret_t*            secret,
                                                ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_attributes_get(vault, secret, attributes);

exit:
  return error;
}

ockam_error_t
ockam_vault_secret_type_set(ockam_vault_t* vault, ockam_vault_secret_t* secret, ockam_vault_secret_type_t type)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_type_set(vault, secret, type);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_destroy(vault, secret);

exit:
  return error;
}

ockam_error_t ockam_vault_ecdh(ockam_vault_t*        vault,
                               ockam_vault_secret_t* privatekey,
                               const uint8_t*        peer_publickey,
                               size_t                peer_publickey_length,
                               ockam_vault_secret_t* shared_secret)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->ecdh(vault, privatekey, peer_publickey, peer_publickey_length, shared_secret);

exit:
  return error;
}

ockam_error_t ockam_vault_hkdf_sha256(ockam_vault_t*        vault,
                                      ockam_vault_secret_t* salt,
                                      ockam_vault_secret_t* input_key_material,
                                      uint8_t               derived_outputs_count,
                                      ockam_vault_secret_t* derived_outputs)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if (vault == 0) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->hkdf_sha256(vault, salt, input_key_material, derived_outputs_count, derived_outputs);

exit:
  return error;
}

ockam_error_t ockam_vault_aead_aes_gcm_encrypt(ockam_vault_t*        vault,
                                               ockam_vault_secret_t* key,
                                               uint16_t              nonce,
                                               const uint8_t*        additional_data,
                                               size_t                additional_data_length,
                                               const uint8_t*        plaintext,
                                               size_t                plaintext_length,
                                               uint8_t*              ciphertext_and_tag,
                                               size_t                ciphertext_and_tag_size,
                                               size_t*               ciphertext_and_tag_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if ((vault == 0) || (ciphertext_and_tag == 0)) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->aead_aes_gcm_encrypt(vault,
                                                key,
                                                nonce,
                                                additional_data,
                                                additional_data_length,
                                                plaintext,
                                                plaintext_length,
                                                ciphertext_and_tag,
                                                ciphertext_and_tag_size,
                                                ciphertext_and_tag_length);
exit:
  return error;
}

ockam_error_t ockam_vault_aead_aes_gcm_decrypt(ockam_vault_t*        vault,
                                               ockam_vault_secret_t* key,
                                               uint16_t              nonce,
                                               const uint8_t*        additional_data,
                                               size_t                additional_data_length,
                                               const uint8_t*        ciphertext_and_tag,
                                               size_t                ciphertext_and_tag_length,
                                               uint8_t*              plaintext,
                                               size_t                plaintext_size,
                                               size_t*               plaintext_length)
{
  ockam_error_t error = ockam_vault_interface_error_none;

  if ((vault == 0) || (plaintext == 0)) {
    error.code = OCKAM_VAULT_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->aead_aes_gcm_decrypt(vault,
                                                key,
                                                nonce,
                                                additional_data,
                                                additional_data_length,
                                                ciphertext_and_tag,
                                                ciphertext_and_tag_length,
                                                plaintext,
                                                plaintext_size,
                                                plaintext_length);

exit:
  return error;
}
