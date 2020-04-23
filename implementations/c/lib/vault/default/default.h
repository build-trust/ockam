/**
 * @file    default.h
 * @brief
 */

#ifndef DEFAULT_H_
#define DEFAULT_H_

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/vault.h"

#include "vault/impl.h"

#define OCKAM_VAULT_FEAT_RANDOM 0x01
#define OCKAM_VAULT_FEAT_SHA256 0x02
#define OCKAM_VAULT_FEAT_SECRET 0x04
#define OCKAM_VAULT_FEAT_AEAD   0x08
#define OCKAM_VAULT_FEAT_ALL    0x0F

/**
 * @struct  ockam_vault_default_common_ctx_t
 * @brief   TBD
 */
typedef struct {
  ockam_memory_t* memory;
  uint32_t        features;
  uint32_t        default_features;
  void*           random_ctx;
  void*           key_ecdh_ctx;
  void*           sha256_ctx;
  void*           hkdf_ctx;
  void*           aes_gcm_ctx;
} ockam_vault_shared_context_t;

/**
 * @struct  ockam_vault_default_attributes_t
 * @brief
 */
typedef struct {
  ockam_memory_t* memory;
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

ockam_error_t vault_default_secret_generate_random(ockam_vault_t*                   vault,
                                                   ockam_vault_secret_t*            secret,
                                                   ockam_vault_secret_attributes_t* secret_attributes);

ockam_error_t vault_default_aead_aes_128_gcm_encrypt(ockam_vault_t*       vault,
                                                     ockam_vault_secret_t key,
                                                     uint16_t             nonce,
                                                     const uint8_t*       additional_data,
                                                     size_t               additional_data_length,
                                                     const uint8_t*       plaintext,
                                                     size_t               plaintext_length,
                                                     uint8_t*             ciphertext_and_tag,
                                                     size_t               ciphertext_and_tag_size,
                                                     size_t*              ciphertext_and_tag_length);

ockam_error_t vault_default_aead_aes_128_gcm_decrypt(ockam_vault_t*       vault,
                                                     ockam_vault_secret_t key,
                                                     uint16_t             nonce,
                                                     const uint8_t*       additional_data,
                                                     size_t               additional_data_length,
                                                     const uint8_t*       ciphertext_and_tag,
                                                     size_t               ciphertext_and_tag_length,
                                                     uint8_t*             plaintext,
                                                     size_t               plaintext_size,
                                                     size_t*              plaintext_length);

#endif
