/**
 * @file    default.h
 * @brief
 */

#ifndef OCKAM_VAULT_DEFAULT_H_
#define OCKAM_VAULT_DEFAULT_H_

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/random.h"
#include "ockam/vault.h"

#include "ockam/vault/impl.h"

extern const char* const OCKAM_VAULT_DEFAULT_ERROR_DOMAIN;

typedef enum {
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_PARAM             = 1,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_ATTRIBUTES        = 2,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_CONTEXT           = 3,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_SIZE              = 4,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_REGENERATE        = 5,
  OCKAM_VAULT_DEFAULT_ERROR_RANDOM_REQUIRED           = 6,
  OCKAM_VAULT_DEFAULT_ERROR_MEMORY_REQUIRED           = 7,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_SECRET_ATTRIBUTES = 8,
  OCKAM_VAULT_DEFAULT_ERROR_SECRET_SIZE_MISMATCH      = 9,
  OCKAM_VAULT_DEFAULT_ERROR_SECRET_GENERATE_FAIL      = 10,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_SECRET_TYPE       = 11,
  OCKAM_VAULT_DEFAULT_ERROR_PUBLIC_KEY_FAIL           = 12,
  OCKAM_VAULT_DEFAULT_ERROR_ECDH_FAIL                 = 13,
  OCKAM_VAULT_DEFAULT_ERROR_INVALID_TAG               = 14,
} ockam_error_code_vault_default_t;

/**
 * @struct  ockam_vault_default_common_ctx_t
 * @brief   TBD
 */
typedef struct {
  ockam_memory_t* memory;
  ockam_random_t* random;
  uint32_t        features;
  uint32_t        default_features;
  void*           random_ctx;
  void*           sha256_ctx;
  void*           hkdf_sha256_ctx;
  void*           aead_aes_gcm_ctx;
} ockam_vault_default_context_t;

/**
 * @struct  ockam_vault_default_attributes_t
 * @brief
 */
typedef struct {
  ockam_memory_t* memory;
  ockam_random_t* random;
  uint32_t        features;
} ockam_vault_default_attributes_t;

ockam_error_t ockam_vault_default_init(ockam_vault_t* vault, ockam_vault_default_attributes_t* vault_attributes);

ockam_error_t vault_default_deinit(ockam_vault_t* vault);

ockam_error_t vault_default_random(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size);

ockam_error_t vault_default_sha256(ockam_vault_t* vault,
                                   const uint8_t* input,
                                   size_t         input_length,
                                   uint8_t*       digest,
                                   size_t         digest_size,
                                   size_t*        digest_length);

ockam_error_t vault_default_secret_generate(ockam_vault_t*                         vault,
                                            ockam_vault_secret_t*                  secret,
                                            const ockam_vault_secret_attributes_t* attributes);

ockam_error_t vault_default_secret_import(ockam_vault_t*                         vault,
                                          ockam_vault_secret_t*                  secret,
                                          const ockam_vault_secret_attributes_t* attributes,
                                          const uint8_t*                         input,
                                          size_t                                 input_length);

ockam_error_t vault_default_secret_export(ockam_vault_t*        vault,
                                          ockam_vault_secret_t* secret,
                                          uint8_t*              output_buffer,
                                          size_t                output_buffer_size,
                                          size_t*               output_buffer_length);

ockam_error_t vault_default_secret_publickey_get(ockam_vault_t*        vault,
                                                 ockam_vault_secret_t* secret,
                                                 uint8_t*              output_buffer,
                                                 size_t                output_buffer_size,
                                                 size_t*               output_buffer_length);

ockam_error_t vault_default_secret_attributes_get(ockam_vault_t*                   vault,
                                                  ockam_vault_secret_t*            secret,
                                                  ockam_vault_secret_attributes_t* attributes);

ockam_error_t
vault_default_secret_type_set(ockam_vault_t* vault, ockam_vault_secret_t* secret, ockam_vault_secret_type_t type);

ockam_error_t vault_default_secret_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret);

ockam_error_t vault_default_ecdh(ockam_vault_t*        vault,
                                 ockam_vault_secret_t* privatekey,
                                 const uint8_t*        peer_publickey,
                                 size_t                peer_publickey_length,
                                 ockam_vault_secret_t* shared_secret);

ockam_error_t vault_default_hkdf_sha256(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* salt,
                                        ockam_vault_secret_t* input_key_material,
                                        uint8_t               derived_outputs_count,
                                        ockam_vault_secret_t* derived_outputs);

ockam_error_t vault_default_aead_aes_gcm_encrypt(ockam_vault_t*        vault,
                                                 ockam_vault_secret_t* key,
                                                 uint16_t              nonce,
                                                 const uint8_t*        additional_data,
                                                 size_t                additional_data_length,
                                                 const uint8_t*        plaintext,
                                                 size_t                plaintext_length,
                                                 uint8_t*              ciphertext_and_tag,
                                                 size_t                ciphertext_and_tag_size,
                                                 size_t*               ciphertext_and_tag_length);

ockam_error_t vault_default_aead_aes_gcm_decrypt(ockam_vault_t*        vault,
                                                 ockam_vault_secret_t* key,
                                                 uint16_t              nonce,
                                                 const uint8_t*        additional_data,
                                                 size_t                additional_data_length,
                                                 const uint8_t*        ciphertext_and_tag,
                                                 size_t                ciphertext_and_tag_length,
                                                 uint8_t*              plaintext,
                                                 size_t                plaintext_size,
                                                 size_t*               plaintext_length);

#endif
