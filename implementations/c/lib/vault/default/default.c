/**
 * @file    default.c
 * @brief   Interface functions for the default Ockam Vault
 */

#include "ockam/memory.h"
#include "ockam/vault.h"
#include "vault/impl.h"

#include "default.h"
#include "bearssl.h"

#define VAULT_DEFAULT_RANDOM_MAX_SIZE 0xFFFF

#define VAULT_DEFAULT_SHA256_DIGEST_SIZE 32u

#define VAULT_DEFAULT_AES_GCM_DECRYPT      0u
#define VAULT_DEFAULT_AES_GCM_ENCRYPT      1u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_128 16u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_192 24u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_256 32u
#define VAULT_DEFAULT_AES_GCM_TAG_SIZE     16u

const char* g_vault_default_random_seed = "ockam_vault_seed";

typedef struct {
  const br_prng_class* br_random;
  void*                br_random_ctx;
} vault_default_random_ctx_t;

typedef struct {
  const br_hash_class* br_sha256;
  void*                br_sha256_ctx;
} vault_default_sha256_ctx_t;

typedef struct {
  const br_ec_impl* br_ec;
  uint32_t          br_curve;
  br_ec_private_key br_private_key;
  unsigned char*    br_private_key_buf;
  size_t            br_private_key_size;
  size_t            br_public_key_size;
} vault_default_secret_ctx_t;

typedef struct {
  br_aes_ct_ctr_keys* br_aes_keys;
  br_gcm_context*     br_aes_gcm_ctx;
} vault_default_aes_gcm_ctx_t;

ockam_error_t vault_default_random_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_random_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_sha256_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_sha256_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_secret_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_secret_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_aead_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_aead_deinit(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_aead_aes_128_gcm(ockam_vault_t*       vault,
                                             uint8_t              encrypt,
                                             ockam_vault_secret_t key,
                                             uint16_t             nonce,
                                             const uint8_t*       additional_data,
                                             size_t               additional_data_length,
                                             const uint8_t*       plaintext,
                                             size_t               plaintext_length,
                                             uint8_t*             ciphertext_and_tag,
                                             size_t               ciphertext_and_tag_size,
                                             size_t*              ciphertext_and_tag_length);

ockam_vault_dispatch_table_t ockam_vault_default_dispatch_table = {
  &vault_default_deinit, &vault_default_random, &vault_default_sha256, 0, 0, 0
};

ockam_error_t ockam_vault_default_init(ockam_vault_t* vault, ockam_vault_default_attributes_t* attributes)
{
  ockam_error_t                 error    = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx      = 0;
  uint32_t                      features = 0;

  if ((vault == 0) || (attributes == 0)) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (attributes->features == 0) {
    if (attributes->memory == 0) {
      error = VAULT_ERROR_INVALID_ATTRIBUTES;
      goto exit;
    }

    error = ockam_memory_alloc(attributes->memory, (uint8_t**) &(vault->context), sizeof(ockam_vault_shared_context_t));
    if (error != OCKAM_ERROR_NONE) { goto exit; }

    ctx      = (ockam_vault_shared_context_t*) vault->context;
    features = OCKAM_VAULT_FEAT_ALL;
  } else {
    if (vault->context == 0) {
      error = VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }

    ctx      = (ockam_vault_shared_context_t*) vault->context;
    features = attributes->features;

    if (ctx->memory == 0) {
      error = VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }
  }

  if (features & OCKAM_VAULT_FEAT_RANDOM) {
    error = vault_default_random_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

  if (features & OCKAM_VAULT_FEAT_SHA256) {
    error = vault_default_sha256_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

#if 0
  if (features & OCKAM_VAULT_FEAT_SECRET) {
    error = vault_default_secret_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

  if (features & OCKAM_VAULT_FEAT_AEAD) {
    error = vault_default_aead_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }
#endif

exit:
  if ((error != OCKAM_ERROR_NONE) && (features == OCKAM_VAULT_FEAT_ALL)) { vault_default_deinit(vault); }

  return error;
}

ockam_error_t vault_default_deinit(ockam_vault_t* vault)
{
  ockam_error_t                 error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx        = 0;
  uint8_t                       delete_ctx = 0;

  if ((vault == 0) || (vault->context == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if (ctx->default_features & OCKAM_VAULT_FEAT_ALL) { delete_ctx = 1; }

  if (ctx->default_features & OCKAM_VAULT_FEAT_RANDOM) { vault_default_random_deinit(ctx); }

  if (ctx->default_features & OCKAM_VAULT_FEAT_SHA256) { vault_default_sha256_deinit(ctx); }

#if 0
  if (ctx->default_features & OCKAM_VAULT_FEAT_SECRET) { vault_default_secret_deinit(ctx); }

  if (ctx->default_features & OCKAM_VAULT_FEAT_AEAD) { vault_default_aead_deinit(ctx); }
#endif

  if (delete_ctx) { ockam_memory_free(ctx->memory, (uint8_t*) ctx, sizeof(ockam_vault_shared_context_t)); }

  vault->context  = 0;
  vault->dispatch = 0;

exit:
  return error;
}

ockam_error_t vault_default_random_init(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error      = OCKAM_ERROR_NONE;
  vault_default_random_ctx_t* random_ctx = 0;

  if (ctx == 0) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc(ctx->memory, (uint8_t**) &random_ctx, sizeof(vault_default_random_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  random_ctx->br_random = &br_hmac_drbg_vtable;

  error =
    ockam_memory_alloc(ctx->memory, (uint8_t**) &(random_ctx->br_random_ctx), random_ctx->br_random->context_size);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  random_ctx->br_random->init(
    random_ctx->br_random_ctx, &br_sha256_vtable, g_vault_default_random_seed, sizeof(g_vault_default_random_seed));

  ctx->random_ctx = random_ctx;
  ctx->default_features |= OCKAM_VAULT_FEAT_RANDOM;

exit:
  return error;
}

ockam_error_t vault_default_random_deinit(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error      = OCKAM_ERROR_NONE;
  vault_default_random_ctx_t* random_ctx = 0;

  if ((ctx == 0) || (ctx->memory == 0) || (ctx->random_ctx == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

  if (random_ctx->br_random_ctx != 0) {
    ockam_memory_free(ctx->memory, (uint8_t*) random_ctx->br_random_ctx, random_ctx->br_random->context_size);
  }

  error = ockam_memory_free(ctx->memory, (uint8_t*) random_ctx, sizeof(vault_default_random_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->random_ctx = 0;
  ctx->default_features &= (!OCKAM_VAULT_FEAT_RANDOM);

exit:
  return error;
}

ockam_error_t vault_default_random(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t                 error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx        = 0;
  vault_default_random_ctx_t*   random_ctx = 0;

  if ((vault == 0) || (vault->context == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->random_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_RANDOM))) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

  if ((random_ctx->br_random == 0) || (random_ctx->br_random_ctx == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (buffer_size > VAULT_DEFAULT_RANDOM_MAX_SIZE) {
    error = VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  random_ctx->br_random->generate(random_ctx->br_random_ctx, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t vault_default_sha256_init(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error      = OCKAM_ERROR_NONE;
  vault_default_sha256_ctx_t* sha256_ctx = 0;

  if (ctx == 0) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc(ctx->memory, (uint8_t**) &sha256_ctx, sizeof(vault_default_sha256_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  sha256_ctx->br_sha256 = &br_sha256_vtable;

  error =
    ockam_memory_alloc(ctx->memory, (uint8_t**) &(sha256_ctx->br_sha256_ctx), sha256_ctx->br_sha256->context_size);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->default_features |= OCKAM_VAULT_FEAT_SHA256;
  ctx->sha256_ctx = sha256_ctx;

exit:
  return error;
}

ockam_error_t vault_default_sha256_deinit(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error      = OCKAM_ERROR_NONE;
  vault_default_sha256_ctx_t* sha256_ctx = 0;

  if ((ctx == 0) || (ctx->sha256_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_SHA256))) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  sha256_ctx = (vault_default_sha256_ctx_t*) ctx->sha256_ctx;

  if (sha256_ctx->br_sha256_ctx != 0) {
    ockam_memory_free(ctx->memory, (uint8_t*) sha256_ctx->br_sha256_ctx, sha256_ctx->br_sha256->context_size);
  }

  error = ockam_memory_free(ctx->memory, (uint8_t*) sha256_ctx, sizeof(vault_default_sha256_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->default_features &= (!OCKAM_VAULT_FEAT_SHA256);

exit:
  return error;
}

ockam_error_t vault_default_sha256(ockam_vault_t* vault,
                                   const uint8_t* input,
                                   size_t         input_length,
                                   uint8_t*       digest,
                                   size_t         digest_size,
                                   size_t*        digest_length)
{
  ockam_error_t                 error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx        = 0;
  vault_default_sha256_ctx_t*   sha256_ctx = 0;

  if ((vault == 0) || (vault->context == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->sha256_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_SHA256))) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  sha256_ctx = (vault_default_sha256_ctx_t*) ctx->sha256_ctx;

  if ((sha256_ctx->br_sha256 == 0) || (sha256_ctx->br_sha256_ctx == 0)) {
    error = VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (digest == 0) {
    error = VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (digest_size != VAULT_DEFAULT_SHA256_DIGEST_SIZE) {
    error = VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  sha256_ctx->br_sha256->init(sha256_ctx->br_sha256_ctx);
  sha256_ctx->br_sha256->update(sha256_ctx->br_sha256_ctx, input, input_length);
  sha256_ctx->br_sha256->out(sha256_ctx->br_sha256_ctx, digest);

  *digest_length = VAULT_DEFAULT_SHA256_DIGEST_SIZE;

exit:
  return error;
}

#if 0


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


/**
 ********************************************************************************************************
 *                                   vault_default_KeyEcdhinit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_KeyEcdhinit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t i = 0;
  size_t size = 0;
  const OckamMemory *memory = 0;
  vault_default_RandomCtx *p_random_ctx = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;
  br_hmac_drbg_context *p_rng = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0) || (p_ctx->random_ctx == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;
  p_random_ctx = p_ctx->random_ctx;
  p_rng = p_random_ctx->br_random_ctx;

  error = memory->Alloc((void **)&p_key_ecdh_ctx, sizeof(vault_default_KeyEcdhCtx));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  switch (p_ctx->ec) {
    case kOckamVaultEcCurve25519:
      p_key_ecdh_ctx->br_ec = &br_ec_c25519_i31;
      p_key_ecdh_ctx->br_curve = BR_EC_curve25519;
      break;

    case kOckamVaultEcP256:
      p_key_ecdh_ctx->br_ec = &br_ec_p256_m31;
      p_key_ecdh_ctx->br_curve = BR_EC_secp256r1;
      break;

    default:
      error = kOckamError;
      goto exit;
      break;
  }

  size = br_ec_keygen(&(p_rng->vtable),      /* Call keygen without a key structure or buffer to   */
                      p_key_ecdh_ctx->br_ec, /* calculate the size of the private key and allocate */
                      0,                     /* buffers appropriately.                             */
                      0, p_key_ecdh_ctx->br_curve);
  if ((size == 0) || (size > BR_EC_KBUF_PRIV_MAX_SIZE)) {
    error = kOckamError;
    goto exit;
  }

  p_key_ecdh_ctx->br_public_key_size = 0;     /* Public key size to be set by Generate or SetPrivate*/
  p_key_ecdh_ctx->br_private_key_size = size; /* Save the size of the private key                   */

  for (i = 0; i < kMaxOckamVaultKey; i++) {
    error = memory->Alloc((void **)&(p_key_ecdh_ctx->br_private_key_buf[i]), p_key_ecdh_ctx->br_private_key_size);
    if (error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  p_ctx->key_ecdh_ctx = p_key_ecdh_ctx;
  p_ctx->features |= OCKAM_VAULT_KEY_ECDH;

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                   vault_default_KeyEcdhdeinit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_KeyEcdhdeinit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t i = 0;
  const OckamMemory *memory = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx->memory == 0) || (p_ctx->key_ecdh_ctx == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;
  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  p_ctx->features &= (!OCKAM_VAULT_KEY_ECDH);

  for (i = 0; i < kMaxOckamVaultKey; i++) {
    if (p_key_ecdh_ctx->br_private_key_buf[i] != 0) {
      memory->Free(p_key_ecdh_ctx->br_private_key_buf[i], p_key_ecdh_ctx->br_private_key_size);
    }
  }

  error = memory->Free(p_key_ecdh_ctx, sizeof(vault_default_KeyEcdhCtx));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                   vault_default_KeyGenerate()
 ********************************************************************************************************
 */

ockam_error_t vault_default_KeyGenerate(ockam_vault_t* vault, OckamVaultKey key_type) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  size_t size = 0;
  vault_default_RandomCtx *p_random_ctx = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;
  br_hmac_drbg_context *p_rng = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (p_ctx->random_ctx == 0) ||
      (!(p_ctx->features & OCKAM_VAULT_RANDOM)) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    error = kOckamError;
    goto exit;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;
  p_random_ctx = p_ctx->random_ctx;
  p_rng = p_random_ctx->br_random_ctx;

  size = br_ec_keygen(&(p_rng->vtable), p_key_ecdh_ctx->br_ec, &(p_key_ecdh_ctx->br_private_key[key_type]),
                      p_key_ecdh_ctx->br_private_key_buf[key_type], p_key_ecdh_ctx->br_curve);
  if (size == 0) {
    error = kOckamError;
    goto exit;
  }

  if (p_key_ecdh_ctx->br_public_key_size == 0) {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, 0, &br_private_key);
    if (size == 0) {
      error = kOckamError;
      goto exit;
    }

    p_key_ecdh_ctx->br_public_key_size = size;
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_default_KeySetPrivate()
 ********************************************************************************************************
 */

ockam_error_t vault_default_KeySetPrivate(ockam_vault_t* vault, OckamVaultKey key_type, uint8_t *p_priv_key,
                                     size_t priv_key_size) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  size_t size = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    error = kOckamError;
    goto exit;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if ((p_priv_key == 0) || (priv_key_size != p_key_ecdh_ctx->br_private_key_size)) {
    error = kOckamError;
    goto exit;
  }

  p_ctx->memory->Copy(p_key_ecdh_ctx->br_private_key_buf[key_type], p_priv_key, priv_key_size);

  if (p_key_ecdh_ctx->br_public_key_size == 0) {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, 0, &br_private_key);
    if (size == 0) {
      error = kOckamError;
      goto exit;
    }

    p_key_ecdh_ctx->br_public_key_size = size;
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_default_KeyGetPublic()
 ********************************************************************************************************
 */

ockam_error_t vault_default_KeyGetPublic(ockam_vault_t* vault, OckamVaultKey key_type, uint8_t *p_pub_key,
                                    size_t pub_key_size) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  size_t size = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    error = kOckamError;
    goto exit;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if ((p_key_ecdh_ctx->br_public_key_size == 0) || (p_key_ecdh_ctx->br_private_key_buf[key_type] == 0)) {
    error = kOckamError;
    goto exit;
  }

  if ((p_pub_key == 0) || (pub_key_size != p_key_ecdh_ctx->br_public_key_size)) {
    error = kOckamError;
    goto exit;
  }

  {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, p_pub_key, &br_private_key);
    if (size == 0) {
      error = kOckamError;
      goto exit;
    }
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_default_Ecdh()
 ********************************************************************************************************
 */

ockam_error_t vault_default_Ecdh(ockam_vault_t* vault, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size,
                            uint8_t *p_ss, size_t ss_size) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  size_t xoff = 0;
  size_t xlen = 0;
  uint32_t ret = 0;
  vault_default_KeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    error = kOckamError;
    goto exit;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if (ss_size != p_key_ecdh_ctx->br_private_key_size) {
    error = kOckamError;
    goto exit;
  }

  p_ctx->memory->Copy(p_ss, p_pub_key, ss_size);

  ret = p_key_ecdh_ctx->br_ec->mul(p_ss, ss_size, p_key_ecdh_ctx->br_private_key_buf[key_type], ss_size,
                                   p_key_ecdh_ctx->br_curve);
  if (ret != 1) {
    error = kOckamError;
    goto exit;
  }

  xoff = p_key_ecdh_ctx->br_ec->xoff(p_key_ecdh_ctx->br_curve, &xlen);
  p_ctx->memory->Move(p_ss, p_ss + xoff, xlen);

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                    vault_default_Hkdfinit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_Hkdfinit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;

  error = memory->Alloc(&(p_ctx->hkdf_ctx), sizeof(br_hkdf_context));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  p_ctx->features |= OCKAM_VAULT_HKDF;

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_default_Hkdfdeinit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_Hkdfdeinit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  const OckamMemory *memory = 0;

  if ((p_ctx->memory == 0) || (p_ctx->hkdf_ctx == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;

  error = memory->Free(p_ctx->hkdf_ctx, sizeof(br_hkdf_context));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  p_ctx->features &= (!OCKAM_VAULT_HKDF);

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_default_Hkdf()
 ********************************************************************************************************
 */

ockam_error_t vault_default_Hkdf(ockam_vault_t* vault, uint8_t *p_salt, size_t salt_size, uint8_t *p_ikm, size_t ikm_size,
                            uint8_t *p_info, size_t info_size, uint8_t *p_out, size_t out_size) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  br_hkdf_context *p_hkdf_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->hkdf_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_HKDF))) {
    error = kOckamError;
    goto exit;
  }

  if ((p_out == 0) || (out_size == 0)) {
    error = kOckamError;
    goto exit;
  }

  if ((p_ikm == 0) != (ikm_size == 0)) {
    error = kOckamError;
    goto exit;
  }

  p_ctx->memory->Set(p_hkdf_ctx, 0, sizeof(br_hkdf_context));

  br_hkdf_init(p_ctx->hkdf_ctx, /* TODO: Absent salt?                                 */
               &br_sha256_vtable, p_salt, salt_size);

  br_hkdf_inject(p_ctx->hkdf_ctx, p_ikm, ikm_size);

  br_hkdf_flip(p_ctx->hkdf_ctx);

  br_hkdf_produce(p_ctx->hkdf_ctx, p_info, info_size, p_out, out_size);

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_default_AesGcminit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_AesGcminit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  vault_default_AesGcmCtx *p_aes_gcm_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;

  error = memory->Alloc((void **)&p_aes_gcm_ctx, sizeof(vault_default_AesGcmCtx));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  error = memory->Alloc((void **)&(p_aes_gcm_ctx->br_aes_keys), sizeof(br_aes_ct_ctr_keys));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  error = memory->Alloc((void **)&(p_aes_gcm_ctx->br_aes_gcm_ctx), sizeof(br_gcm_context));
  if (error != OCKAM_ERROR_NONE) {
    memory->Free(p_aes_gcm_ctx->br_aes_keys, sizeof(br_aes_ct_ctr_keys));
    goto exit;
  }

  p_ctx->features |= OCKAM_VAULT_AES_GCM;
  p_ctx->aes_gcm_ctx = p_aes_gcm_ctx;

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_default_AesGcmdeinit()
 ********************************************************************************************************
 */

ockam_error_t vault_default_AesGcmdeinit(ockam_vault_t* vault) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  vault_default_AesGcmCtx *p_aes_gcm_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0) || (p_ctx->aes_gcm_ctx == 0)) {
    error = kOckamError;
    goto exit;
  }

  memory = p_ctx->memory;

  if (p_aes_gcm_ctx->br_aes_gcm_ctx != 0) {
    memory->Free(p_aes_gcm_ctx->br_aes_gcm_ctx, sizeof(br_gcm_context));
  }

  if (p_aes_gcm_ctx->br_aes_keys != 0) {
    memory->Free(p_aes_gcm_ctx->br_aes_keys, sizeof(br_aes_ct_ctr_keys));
  }

  error = memory->Free(p_aes_gcm_ctx, sizeof(vault_default_AesGcmCtx));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  p_ctx->features &= (!OCKAM_VAULT_AES_GCM);

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                     vault_default_AesGcm()
 ********************************************************************************************************
 */

ockam_error_t vault_default_AesGcm(ockam_vault_t* vault, int encrypt, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                              size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                              uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  ockam_error_t error = OCKAM_ERROR_NONE;
  vault_default_AesGcmCtx *p_aes_gcm_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->aes_gcm_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_AES_GCM))) {
    error = kOckamError;
    goto exit;
  }

  p_aes_gcm_ctx = p_ctx->aes_gcm_ctx;

  if ((p_aes_gcm_ctx->br_aes_keys == 0) || (p_aes_gcm_ctx->br_aes_gcm_ctx == 0)) {
    error = kOckamError;
    goto exit;
  }

  if ((p_key == 0) ||
      ((key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_128) && (key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_192) &&
       (key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_256))) {
    error = kOckamError;
    goto exit;
  }

  if (tag_size != VAULT_DEFAULT_AES_GCM_TAG_SIZE) {
    error = kOckamError;
    goto exit;
  }

  if ((p_input == 0) != (input_size == 0)) {
    error = kOckamError;
    goto exit;
  }

  if ((p_output == 0) != (output_size == 0)) {
    error = kOckamError;
    goto exit;
  }

  if (input_size != output_size) {
    error = kOckamError;
    goto exit;
  }

  br_aes_ct_ctr_init(p_aes_gcm_ctx->br_aes_keys, p_key, key_size);

  br_gcm_init(p_aes_gcm_ctx->br_aes_gcm_ctx, &(p_aes_gcm_ctx->br_aes_keys->vtable), br_ghash_ctmul32);

  br_gcm_reset(p_aes_gcm_ctx->br_aes_gcm_ctx, p_iv, iv_size);

  br_gcm_aad_inject(p_aes_gcm_ctx->br_aes_gcm_ctx, p_aad, aad_size);

  br_gcm_flip(p_aes_gcm_ctx->br_aes_gcm_ctx);

  p_ctx->memory->Copy(p_output, p_input, input_size);

  br_gcm_run(p_aes_gcm_ctx->br_aes_gcm_ctx, encrypt, p_output, output_size);

  if (encrypt == VAULT_DEFAULT_AES_GCM_ENCRYPT) {
    br_gcm_get_tag(p_aes_gcm_ctx->br_aes_gcm_ctx, p_tag);
  } else {
    if (!(br_gcm_check_tag(p_aes_gcm_ctx->br_aes_gcm_ctx, p_tag))) {
      error = kOckamError;
      goto exit;
    }
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_default_AesGcmDecrypt()
 ********************************************************************************************************
 */

ockam_error_t vault_default_AesGcmEncrypt(ockam_vault_t* vault, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                     size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                     uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return vault_default_AesGcm(p_ctx, VAULT_DEFAULT_AES_GCM_ENCRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size,
                            p_tag, tag_size, p_input, input_size, p_output, output_size);
}

/**
 ********************************************************************************************************
 *                                  vault_default_AesGcmDecrypt()
 ********************************************************************************************************
 */

ockam_error_t vault_default_AesGcmDecrypt(ockam_vault_t* vault, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                     size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                     uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return vault_default_AesGcm(p_ctx, VAULT_DEFAULT_AES_GCM_DECRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size,
                            p_tag, tag_size, p_input, input_size, p_output, output_size);
}

#endif
