#include "ockam/error.h"
#include "ockam/vault.h"

#include "vault/impl.h"

ockam_error_t ockam_vault_deinit(ockam_vault_t* vault)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (vault == 0) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->deinit(vault);

exit:
  return error;
}

ockam_error_t ockam_vault_random_bytes_generate(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || buffer == 0) {
    error = VAULT_ERROR_INVALID_PARAM;
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
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || (digest == 0)) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->sha256(vault, input, input_length, digest, digest_size, digest_length);

exit:
  return error;
}

ockam_error_t ockam_vault_secret_generate_random(ockam_vault_t*                   vault,
                                                 ockam_vault_secret_t*            secret,
                                                 ockam_vault_secret_attributes_t* secret_attributes)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (vault == 0) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->secret_generate_random(vault, secret, secret_attributes);

exit:
  return error;
}

ockam_error_t ockam_vault_aead_aes_128_gcm_encrypt(ockam_vault_t*       vault,
                                                   ockam_vault_secret_t key,
                                                   uint16_t             nonce,
                                                   const uint8_t*       additional_data,
                                                   size_t               additional_data_length,
                                                   const uint8_t*       plaintext,
                                                   size_t               plaintext_length,
                                                   uint8_t*             ciphertext_and_tag,
                                                   size_t               ciphertext_and_tag_size,
                                                   size_t*              ciphertext_and_tag_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || (ciphertext_and_tag == 0)) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->aead_aes_128_gcm_encrypt(vault,
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

ockam_error_t ockam_vault_aead_aes_128_gcm_decrypt(ockam_vault_t*       vault,
                                                   ockam_vault_secret_t key,
                                                   uint16_t             nonce,
                                                   const uint8_t*       additional_data,
                                                   size_t               additional_data_length,
                                                   const uint8_t*       ciphertext_and_tag,
                                                   size_t               ciphertext_and_tag_length,
                                                   uint8_t*             plaintext,
                                                   size_t               plaintext_size,
                                                   size_t*              plaintext_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || (plaintext == 0)) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = vault->dispatch->aead_aes_128_gcm_decrypt(vault,
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
