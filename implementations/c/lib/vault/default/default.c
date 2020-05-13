/**
 * @file    default.c
 * @brief   Interface functions for the default Ockam Vault
 */

#include "ockam/memory.h"
#include "ockam/vault.h"
#include "vault/impl.h"

#include "default.h"
#include "bearssl.h"

#define VAULT_DEFAULT_RANDOM_SEED_BYTES      32u
#define VAULT_DEFAULT_RANDOM_MAX_SIZE        0xFFFF
#define VAULT_DEFAULT_SHA256_DIGEST_SIZE     32u
#define VAULT_DEFAULT_AEAD_AES_GCM_DECRYPT   0u
#define VAULT_DEFAULT_AEAD_AES_GCM_ENCRYPT   1u
#define VAULT_DEFAULT_AEAD_AES_GCM_IV_SIZE   12u
#define VAULT_DEFAULT_AEAD_AES_GCM_IV_OFFSET 10u

typedef struct {
  const br_prng_class* br_random;
  void*                br_random_ctx;
} vault_default_random_ctx_t;

typedef struct {
  const br_hash_class* br_sha256;
  void*                br_sha256_ctx;
} vault_default_sha256_ctx_t;

typedef struct {
  const br_ec_impl* ec;
  uint32_t          curve;
  uint8_t*          private_key;
  size_t            private_key_size;
  size_t            ockam_public_key_size;
} vault_default_secret_ec_ctx_t;

typedef struct {
  uint8_t* key;
  size_t   key_size;
  size_t   buffer_size;
} vault_default_secret_key_ctx_t;

typedef struct {
  br_gcm_context*     br_aes_gcm_ctx;
  br_aes_ct_ctr_keys* br_aes_key;
} vault_default_aead_aes_gcm_ctx_t;

ockam_error_t vault_default_secret_ec_create(ockam_vault_t*                         vault,
                                             ockam_vault_secret_t*                  secret,
                                             const ockam_vault_secret_attributes_t* attributes,
                                             uint8_t                                generate,
                                             const uint8_t*                         input,
                                             size_t                                 input_length);

ockam_error_t vault_default_secret_key_create(ockam_vault_t*                         vault,
                                              ockam_vault_secret_t*                  secret,
                                              const ockam_vault_secret_attributes_t* attributes,
                                              uint8_t                                generate,
                                              const uint8_t*                         input,
                                              size_t                                 input_length);

ockam_error_t vault_default_secret_ec_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret);
ockam_error_t vault_default_secret_key_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret);

ockam_error_t vault_default_random_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_random_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_sha256_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_sha256_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_hkdf_sha256_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_hkdf_sha256_deinit(ockam_vault_shared_context_t* ctx);

ockam_error_t vault_default_aead_aes_gcm_init(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_aead_aes_gcm_deinit(ockam_vault_shared_context_t* ctx);
ockam_error_t vault_default_aead_aes_gcm(ockam_vault_t*        vault,
                                         uint8_t               encrypt,
                                         ockam_vault_secret_t* key,
                                         uint16_t              nonce,
                                         const uint8_t*        additional_data,
                                         size_t                additional_data_length,
                                         const uint8_t*        input,
                                         size_t                input_length,
                                         uint8_t*              output,
                                         size_t                output_size,
                                         size_t*               output_length);

ockam_vault_dispatch_table_t vault_default_dispatch_table = {
  &vault_default_deinit,
  &vault_default_random,
  &vault_default_sha256,
  &vault_default_secret_generate,
  &vault_default_secret_import,
  &vault_default_secret_export,
  &vault_default_secret_publickey_get,
  &vault_default_secret_attributes_get,
  &vault_default_secret_type_set,
  &vault_default_secret_destroy,
  &vault_default_ecdh,
  &vault_default_hkdf_sha256,
  &vault_default_aead_aes_gcm_encrypt,
  &vault_default_aead_aes_gcm_decrypt,
};

ockam_error_t ockam_vault_default_init(ockam_vault_t* vault, ockam_vault_default_attributes_t* attributes)
{
  ockam_error_t                 error    = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx      = 0;
  uint32_t                      features = 0;

  if ((vault == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (attributes->features == 0) {
    if (attributes->memory == 0) {
      error = OCKAM_VAULT_ERROR_INVALID_ATTRIBUTES;
      goto exit;
    }

    error =
      ockam_memory_alloc_zeroed(attributes->memory, (void**) &(vault->context), sizeof(ockam_vault_shared_context_t));
    if (error != OCKAM_ERROR_NONE) { goto exit; }

    ctx         = (ockam_vault_shared_context_t*) vault->context;
    ctx->memory = attributes->memory;
    ctx->random = attributes->random;

    vault->dispatch = &vault_default_dispatch_table;

    features = OCKAM_VAULT_FEAT_ALL;
  } else {
    if (vault->context == 0) {
      error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }

    ctx      = (ockam_vault_shared_context_t*) vault->context;
    features = attributes->features;

    if (ctx->memory == 0) {
      error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }

    if ((ctx->random == 0) && ((features & OCKAM_VAULT_FEAT_RANDOM) || (features & OCKAM_VAULT_FEAT_SECRET_ECDH))) {
      error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
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

  if (features & OCKAM_VAULT_FEAT_SECRET_ECDH) { ctx->default_features |= OCKAM_VAULT_FEAT_SECRET_ECDH; }

  if (features & OCKAM_VAULT_FEAT_HKDF_SHA256) {
    error = vault_default_hkdf_sha256_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

  if (features & OCKAM_VAULT_FEAT_AEAD_AES_GCM) {
    error = vault_default_aead_aes_gcm_init(ctx);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if (ctx->default_features & OCKAM_VAULT_FEAT_ALL) { delete_ctx = 1; }

  if (ctx->default_features & OCKAM_VAULT_FEAT_RANDOM) { vault_default_random_deinit(ctx); }

  if (ctx->default_features & OCKAM_VAULT_FEAT_SHA256) { vault_default_sha256_deinit(ctx); }

  if (ctx->default_features & OCKAM_VAULT_FEAT_SECRET_ECDH) {
    ctx->default_features &= (!OCKAM_VAULT_FEAT_SECRET_ECDH);
  }

  if (ctx->default_features & OCKAM_VAULT_FEAT_HKDF_SHA256) { vault_default_hkdf_sha256_deinit(ctx); }

  if (ctx->default_features & OCKAM_VAULT_FEAT_AEAD_AES_GCM) { vault_default_aead_aes_gcm_deinit(ctx); }

  if (delete_ctx) { ockam_memory_free(ctx->memory, ctx, sizeof(ockam_vault_shared_context_t)); }

  vault->context  = 0;
  vault->dispatch = 0;

exit:
  return error;
}

ockam_error_t vault_default_random_init(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error                                   = OCKAM_ERROR_NONE;
  vault_default_random_ctx_t* random_ctx                              = 0;
  uint8_t                     buffer[VAULT_DEFAULT_RANDOM_SEED_BYTES] = { 0 };

  if ((ctx == 0) || (ctx->random == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_random_get_bytes(ctx->random, &buffer[0], VAULT_DEFAULT_RANDOM_SEED_BYTES);
  if (error != OCKAM_ERROR_NONE) { goto exit; };

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &random_ctx, sizeof(vault_default_random_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  random_ctx->br_random = &br_hmac_drbg_vtable;

  error =
    ockam_memory_alloc_zeroed(ctx->memory, (void**) &(random_ctx->br_random_ctx), random_ctx->br_random->context_size);
  if (error != OCKAM_ERROR_NONE) {
    ockam_memory_free(ctx->memory, random_ctx, sizeof(vault_default_random_ctx_t));
    goto exit;
  }

  random_ctx->br_random->init(
    random_ctx->br_random_ctx, &br_sha256_vtable, &buffer[0], sizeof(VAULT_DEFAULT_RANDOM_SEED_BYTES));

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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

  if (random_ctx->br_random_ctx != 0) {
    ockam_memory_free(ctx->memory, random_ctx->br_random_ctx, random_ctx->br_random->context_size);
  }

  error = ockam_memory_free(ctx->memory, random_ctx, sizeof(vault_default_random_ctx_t));
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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->random_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_RANDOM))) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

  if ((random_ctx->br_random == 0) || (random_ctx->br_random_ctx == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (buffer_size > VAULT_DEFAULT_RANDOM_MAX_SIZE) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &sha256_ctx, sizeof(vault_default_sha256_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  sha256_ctx->br_sha256 = &br_sha256_vtable;

  error =
    ockam_memory_alloc_zeroed(ctx->memory, (void**) &(sha256_ctx->br_sha256_ctx), sha256_ctx->br_sha256->context_size);
  if (error != OCKAM_ERROR_NONE) {
    ockam_memory_free(ctx->memory, sha256_ctx, sizeof(vault_default_sha256_ctx_t));
    goto exit;
  }

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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  sha256_ctx = (vault_default_sha256_ctx_t*) ctx->sha256_ctx;

  if (sha256_ctx->br_sha256_ctx != 0) {
    ockam_memory_free(ctx->memory, sha256_ctx->br_sha256_ctx, sha256_ctx->br_sha256->context_size);
  }

  error = ockam_memory_free(ctx->memory, sha256_ctx, sizeof(vault_default_sha256_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->default_features &= (!OCKAM_VAULT_FEAT_SHA256);
  ctx->sha256_ctx = 0;

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
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->sha256_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_SHA256))) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  sha256_ctx = (vault_default_sha256_ctx_t*) ctx->sha256_ctx;

  if ((sha256_ctx->br_sha256 == 0) || (sha256_ctx->br_sha256_ctx == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (digest == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (digest_size != VAULT_DEFAULT_SHA256_DIGEST_SIZE) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  sha256_ctx->br_sha256->init(sha256_ctx->br_sha256_ctx);
  sha256_ctx->br_sha256->update(sha256_ctx->br_sha256_ctx, input, input_length);
  sha256_ctx->br_sha256->out(sha256_ctx->br_sha256_ctx, digest);

  *digest_length = VAULT_DEFAULT_SHA256_DIGEST_SIZE;

exit:
  return error;
}

ockam_error_t vault_default_secret_generate(ockam_vault_t*                         vault,
                                            ockam_vault_secret_t*                  secret,
                                            const ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (secret->context != 0) {
    if (secret->attributes.type != attributes->type) {
      error = OCKAM_VAULT_ERROR_INVALID_REGENERATE; // TODO is this correct?
      goto exit;
    }
  }

  switch (attributes->type) {
  case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY:
  case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY:
    error = vault_default_secret_ec_create(vault, secret, attributes, 1, 0, 0);
    break;

  case OCKAM_VAULT_SECRET_TYPE_AES128_KEY:
  case OCKAM_VAULT_SECRET_TYPE_AES256_KEY:
  case OCKAM_VAULT_SECRET_TYPE_BUFFER:
    error = vault_default_secret_key_create(vault, secret, attributes, 1, 0, 0);
    break;

  default:
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    break;
  }

exit:
  return error;
}

ockam_error_t vault_default_secret_import(ockam_vault_t*                         vault,
                                          ockam_vault_secret_t*                  secret,
                                          const ockam_vault_secret_attributes_t* attributes,
                                          const uint8_t*                         input,
                                          size_t                                 input_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (secret->context != 0) {
    if (secret->attributes.type != attributes->type) {
      error = OCKAM_VAULT_ERROR_INVALID_REGENERATE; // TODO is this correct?
      goto exit;
    }
  }

  switch (attributes->type) {
  case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY:
  case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY:
    error = vault_default_secret_ec_create(vault, secret, attributes, 0, input, input_length);
    break;

  case OCKAM_VAULT_SECRET_TYPE_AES128_KEY:
  case OCKAM_VAULT_SECRET_TYPE_AES256_KEY:
  case OCKAM_VAULT_SECRET_TYPE_BUFFER:
    error = vault_default_secret_key_create(vault, secret, attributes, 0, input, input_length);
    break;

  default:
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    break;
  }

exit:
  return error;
}

ockam_error_t vault_default_secret_ec_create(ockam_vault_t*                         vault,
                                             ockam_vault_secret_t*                  secret,
                                             const ockam_vault_secret_attributes_t* attributes,
                                             uint8_t                                generate,
                                             const uint8_t*                         input,
                                             size_t                                 input_length)
{
  ockam_error_t                  error         = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*  ctx           = 0;
  vault_default_random_ctx_t*    random_ctx    = 0;
  vault_default_secret_ec_ctx_t* secret_ctx    = 0;
  br_hmac_drbg_context*          br_random_ctx = 0;
  size_t                         size          = 0;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((input == 0) != (input_length == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->random_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_RANDOM))) {
    error = OCKAM_VAULT_ERROR_DEFAULT_RANDOM_REQUIRED;
    goto exit;
  }

  random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

  if ((random_ctx->br_random == 0) || (random_ctx->br_random_ctx == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  br_random_ctx = random_ctx->br_random_ctx;

  if (ctx->memory == 0) {
    error = OCKAM_VAULT_ERROR_MEMORY_REQUIRED;
    goto exit;
  }

  if ((attributes->purpose != OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT) ||
      (attributes->persistence != OCKAM_VAULT_SECRET_EPHEMERAL)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_ATTRIBUTES;
    goto exit;
  }

  if (secret->context == 0) {
    error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &secret_ctx, sizeof(vault_default_secret_ec_ctx_t));
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  } else {
    secret_ctx = (vault_default_secret_ec_ctx_t*) secret->context;
  }

  ockam_memory_set(ctx->memory, &(secret->attributes), 0, sizeof(ockam_vault_secret_attributes_t));

  switch (attributes->type) {
  case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY:
    secret_ctx->ec    = &br_ec_p256_m31;
    secret_ctx->curve = BR_EC_secp256r1;

  case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY:
    secret_ctx->ec    = &br_ec_c25519_i31;
    secret_ctx->curve = BR_EC_curve25519;
    break;

  default:
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
    break;
  }

  size = br_ec_keygen(&(br_random_ctx->vtable), /* Call keygen without a key structure or buffer to     */
                      secret_ctx->ec,           /* calculate the size of the private key                */
                      0,
                      0,
                      secret_ctx->curve);
  if ((size == 0) || (size > BR_EC_KBUF_PRIV_MAX_SIZE)) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  if (secret_ctx->private_key_size != 0) {
    if (secret_ctx->private_key_size != size) {
      error = OCKAM_VAULT_ERROR_SECRET_SIZE_MISMATCH;
      goto exit;
    }
  } else {
    if ((input_length != size) && (input_length != 0)) {
      error = OCKAM_VAULT_ERROR_SECRET_SIZE_MISMATCH;
      goto exit;
    } else {
      secret_ctx->private_key_size = size;
    }
  }

  if (secret_ctx->private_key == 0) {
    error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &(secret_ctx->private_key), secret_ctx->private_key_size);
    if (error != OCKAM_ERROR_NONE) {
      ockam_memory_free(ctx->memory, secret_ctx, sizeof(vault_default_secret_ec_ctx_t));
      secret->context = 0;
      goto exit;
    }
  }

  if (input == 0) {
    size = br_ec_keygen(&(br_random_ctx->vtable), secret_ctx->ec, 0, secret_ctx->private_key, secret_ctx->curve);
    if (size == 0) {
      error = OCKAM_VAULT_ERROR_KEYGEN_FAIL;
      goto exit;
    }
  } else {
    ockam_memory_copy(ctx->memory, secret_ctx->private_key, input, input_length);
  }

  {
    const br_ec_private_key private_key = { .curve = secret_ctx->curve,
                                            .xlen  = secret_ctx->private_key_size,
                                            .x     = secret_ctx->private_key };

    size = br_ec_compute_pub(secret_ctx->ec, 0, 0, &private_key);
    if (size == 0) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }

    secret_ctx->ockam_public_key_size = size;
  }

  ockam_memory_copy(ctx->memory, &(secret->attributes), attributes, sizeof(ockam_vault_secret_attributes_t));

  secret->attributes.length = secret_ctx->private_key_size; /* User-supplied length is always ignored for EC keys,   */
  secret->context           = secret_ctx;                   /* instead we save the private key length.               */

exit:
  return error;
}

ockam_error_t vault_default_secret_key_create(ockam_vault_t*                         vault,
                                              ockam_vault_secret_t*                  secret,
                                              const ockam_vault_secret_attributes_t* attributes,
                                              uint8_t                                generate,
                                              const uint8_t*                         input,
                                              size_t                                 input_length)
{
  ockam_error_t                   error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*   ctx        = 0;
  vault_default_random_ctx_t*     random_ctx = 0;
  vault_default_secret_key_ctx_t* secret_ctx = 0;
  size_t                          size       = 0;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((input == 0) && (input_length != 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if (generate) {
    if ((ctx->random_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_RANDOM))) {
      error = OCKAM_VAULT_ERROR_DEFAULT_RANDOM_REQUIRED;
      goto exit;
    }

    random_ctx = (vault_default_random_ctx_t*) ctx->random_ctx;

    if ((random_ctx->br_random == 0) || (random_ctx->br_random_ctx == 0)) {
      error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }
  }

  if (ctx->memory == 0) {
    error = OCKAM_VAULT_ERROR_MEMORY_REQUIRED;
    goto exit;
  }

  if ((attributes->purpose != OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT) ||
      (attributes->persistence != OCKAM_VAULT_SECRET_EPHEMERAL)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_ATTRIBUTES;
    goto exit;
  }

  if (secret->context == 0) {
    error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &secret_ctx, sizeof(vault_default_secret_key_ctx_t));
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  } else {
    secret_ctx = (vault_default_secret_key_ctx_t*) secret->context;
  }

  if ((secret_ctx->key != 0) && (attributes->length != secret_ctx->key_size)) {
    ockam_memory_free(ctx->memory, secret_ctx->key, secret_ctx->key_size);
    secret_ctx->key = 0;
  }

  secret_ctx->key_size    = attributes->length;
  secret_ctx->buffer_size = attributes->length;

  if (secret_ctx->key == 0) {
    error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &(secret_ctx->key), secret_ctx->key_size);
    if (error != OCKAM_ERROR_NONE) {
      ockam_memory_free(ctx->memory, secret_ctx, sizeof(vault_default_secret_ec_ctx_t));
      secret->context = 0;
      goto exit;
    }
  }

  if (generate) {
    error = vault_default_random(vault, secret_ctx->key, secret_ctx->key_size);
    if (error != OCKAM_ERROR_NONE) {
      vault_default_secret_destroy(vault, secret);
      goto exit;
    }
  } else if (input != 0) {
    if (input_length > secret_ctx->key_size) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }

    ockam_memory_copy(ctx->memory, secret_ctx->key, input, input_length);
  }

  ockam_memory_copy(ctx->memory, &(secret->attributes), attributes, sizeof(ockam_vault_secret_attributes_t));

  secret->context = secret_ctx;

exit:
  return error;
}

ockam_error_t vault_default_secret_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret)
{
  ockam_error_t                   error          = OCKAM_ERROR_NONE;
  vault_default_secret_ec_ctx_t*  secret_ec_ctx  = 0;
  vault_default_secret_key_ctx_t* secret_key_ctx = 0;

  if ((vault == 0) || (secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  switch (secret->attributes.type) {
  case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY:
  case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY:
    error = vault_default_secret_ec_destroy(vault, secret);
    break;

  case OCKAM_VAULT_SECRET_TYPE_AES128_KEY:
  case OCKAM_VAULT_SECRET_TYPE_AES256_KEY:
  case OCKAM_VAULT_SECRET_TYPE_BUFFER:
    error = vault_default_secret_key_destroy(vault, secret);
    break;

  default:
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    break;
  }

exit:
  return error;
}

ockam_error_t vault_default_secret_ec_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret)
{
  ockam_error_t                  error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*  ctx        = 0;
  vault_default_secret_ec_ctx_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_ec_ctx_t*) secret->context;

  if (secret_ctx->private_key != 0) {
    ockam_memory_free(ctx->memory, secret_ctx->private_key, secret_ctx->private_key_size);
  }

  ockam_memory_free(ctx->memory, secret_ctx, sizeof(vault_default_secret_ec_ctx_t));
  ockam_memory_set(ctx->memory, &(secret->attributes), 0, sizeof(ockam_vault_secret_attributes_t));

  secret->context = 0;

exit:
  return error;
}

ockam_error_t vault_default_secret_key_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret)
{
  ockam_error_t                   error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*   ctx        = 0;
  vault_default_secret_key_ctx_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_key_ctx_t*) secret->context;

  if (secret_ctx->key != 0) { ockam_memory_free(ctx->memory, secret_ctx->key, secret_ctx->buffer_size); }

  ockam_memory_free(ctx->memory, secret_ctx, sizeof(vault_default_secret_key_ctx_t));
  ockam_memory_set(ctx->memory, &(secret->attributes), 0, sizeof(ockam_vault_secret_attributes_t));

  secret->context = 0;

exit:
  return error;
}

ockam_error_t vault_default_secret_export(ockam_vault_t*        vault,
                                          ockam_vault_secret_t* secret,
                                          uint8_t*              output_buffer,
                                          size_t                output_buffer_size,
                                          size_t*               output_buffer_length)
{
  ockam_error_t                   error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*   ctx        = 0;
  vault_default_secret_key_ctx_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0) || (output_buffer == 0) || (output_buffer_length == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->attributes.length > output_buffer_size) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_key_ctx_t*) secret->context;

  error = ockam_memory_copy(ctx->memory, output_buffer, secret_ctx->key, secret_ctx->key_size);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  *output_buffer_length = secret_ctx->key_size;

exit:
  return error;
}

ockam_error_t vault_default_secret_publickey_get(ockam_vault_t*        vault,
                                                 ockam_vault_secret_t* secret,
                                                 uint8_t*              output_buffer,
                                                 size_t                output_buffer_size,
                                                 size_t*               output_buffer_length)
{
  ockam_error_t                  error      = OCKAM_ERROR_NONE;
  vault_default_secret_ec_ctx_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0) || (output_buffer == 0) || (output_buffer_length == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_ec_ctx_t*) secret->context;

  if (secret_ctx->ockam_public_key_size > output_buffer_size) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  {
    size_t                  size           = 0;
    const br_ec_private_key br_private_key = { .curve = secret_ctx->curve,
                                               .xlen  = secret_ctx->private_key_size,
                                               .x     = secret_ctx->private_key };

    size = br_ec_compute_pub(secret_ctx->ec, 0, output_buffer, &br_private_key);
    if (size == 0) {
      error = OCKAM_VAULT_ERROR_PUBLIC_KEY_FAIL;
      goto exit;
    }
  }

  *output_buffer_length = secret_ctx->ockam_public_key_size;

exit:
  return error;
}

ockam_error_t vault_default_secret_attributes_get(ockam_vault_t*                   vault,
                                                  ockam_vault_secret_t*            secret,
                                                  ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t                 error = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t* ctx   = 0;
  size_t                        size  = 0;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  error = ockam_memory_copy(ctx->memory, attributes, &(secret->attributes), sizeof(ockam_vault_secret_attributes_t));

exit:
  return error;
}

ockam_error_t
vault_default_secret_type_set(ockam_vault_t* vault, ockam_vault_secret_t* secret, ockam_vault_secret_type_t type)
{
  ockam_error_t                   error      = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*   ctx        = 0;
  vault_default_secret_key_ctx_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_key_ctx_t*) secret->context;

  if (type == OCKAM_VAULT_SECRET_TYPE_AES128_KEY) {
    if (secret_ctx->key_size < OCKAM_VAULT_AES128_KEY_LENGTH) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }

    secret->attributes.type   = type;
    secret->attributes.length = OCKAM_VAULT_AES128_KEY_LENGTH;
    secret_ctx->key_size      = OCKAM_VAULT_AES128_KEY_LENGTH;
  } else if (type == OCKAM_VAULT_SECRET_TYPE_AES256_KEY) {
    if (secret_ctx->key_size < OCKAM_VAULT_AES256_KEY_LENGTH) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }

    secret->attributes.type   = type;
    secret->attributes.length = OCKAM_VAULT_AES256_KEY_LENGTH;
    secret_ctx->key_size      = OCKAM_VAULT_AES256_KEY_LENGTH;
  } else if (type == OCKAM_VAULT_SECRET_TYPE_BUFFER) {
    secret->attributes.type = type;
  } else {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
  }

exit:
  return error;
}

ockam_error_t vault_default_ecdh(ockam_vault_t*        vault,
                                 ockam_vault_secret_t* privatekey,
                                 const uint8_t*        peer_publickey,
                                 size_t                peer_publickey_length,
                                 ockam_vault_secret_t* shared_secret)
{
  ockam_error_t                   error          = OCKAM_ERROR_NONE;
  int                             ret            = 0;
  const uint8_t*                  publickey      = 0;
  ockam_vault_shared_context_t*   ctx            = 0;
  vault_default_secret_ec_ctx_t*  secret_ec_ctx  = 0;
  vault_default_secret_key_ctx_t* secret_key_ctx = 0;

  if ((vault == 0) || (privatekey == 0) || (peer_publickey == 0) || (shared_secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((privatekey->attributes.type != OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) &&
      (privatekey->attributes.type != OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  {
    const ockam_vault_secret_attributes_t attributes = { .length      = peer_publickey_length,
                                                         .type        = OCKAM_VAULT_SECRET_TYPE_BUFFER,
                                                         .purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                         .persistence = OCKAM_VAULT_SECRET_EPHEMERAL };

    error = vault_default_secret_key_create(vault, shared_secret, &attributes, 0, 0, 0);
    if (error != OCKAM_ERROR_NONE) { goto exit; }
  }

  if ((privatekey->context == 0) || (shared_secret->context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ec_ctx  = (vault_default_secret_ec_ctx_t*) privatekey->context;
  secret_key_ctx = (vault_default_secret_key_ctx_t*) shared_secret->context;

  ockam_memory_copy(ctx->memory, secret_key_ctx->key, peer_publickey, peer_publickey_length);

  ret = secret_ec_ctx->ec->mul(secret_key_ctx->key,
                               shared_secret->attributes.length,
                               secret_ec_ctx->private_key,
                               secret_ec_ctx->private_key_size,
                               secret_ec_ctx->curve);
  if (ret != 1) {
    error = OCKAM_VAULT_ERROR_ECDH_FAIL;
    goto exit;
  } else {
    secret_key_ctx->key_size = OCKAM_VAULT_SHARED_SECRET_LENGTH;
  }

  // TODO : Is this needed?
  // xoff = p_key_ecdh_ctx->br_ec->xoff(p_key_ecdh_ctx->br_curve, &xlen);
  // ockam_memory_move(ctx->memory, p_ss, p_ss + xoff, xlen);

exit:
  return error;
}

ockam_error_t vault_default_hkdf_sha256_init(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t               error      = OCKAM_ERROR_NONE;
  vault_default_sha256_ctx_t* sha256_ctx = 0;

  if (ctx == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &(ctx->hkdf_sha256_ctx), sizeof(br_hkdf_context));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->default_features |= OCKAM_VAULT_FEAT_HKDF_SHA256;

exit:
  return error;
}

ockam_error_t vault_default_hkdf_sha256_deinit(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_HKDF_SHA256))) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_free(ctx->memory, ctx->hkdf_sha256_ctx, sizeof(br_hkdf_context));

  ctx->hkdf_sha256_ctx = 0;
  ctx->default_features &= (!OCKAM_VAULT_FEAT_HKDF_SHA256);

exit:
  return error;
}

ockam_error_t vault_default_hkdf_sha256(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* salt,
                                        ockam_vault_secret_t* input_key_material,
                                        uint8_t               derived_outputs_count,
                                        ockam_vault_secret_t* derived_outputs)
{
  ockam_error_t                   error       = OCKAM_ERROR_NONE;
  br_hkdf_context*                br_hkdf_ctx = 0;
  ockam_vault_shared_context_t*   ctx         = 0;
  vault_default_secret_key_ctx_t* secret_ctx  = 0;

  if ((vault == 0) || (salt == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((salt->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER) &&
      (salt->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
      (salt->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (input_key_material != 0) {
    if ((input_key_material->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER) &&
        (input_key_material->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
        (input_key_material->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
      error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    }
  }

  if (vault->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->hkdf_sha256_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_HKDF_SHA256))) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  br_hkdf_ctx = (br_hkdf_context*) ctx->hkdf_sha256_ctx;

  ockam_memory_set(ctx->memory, br_hkdf_ctx, 0, sizeof(br_hkdf_context));

  if (salt->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_key_ctx_t*) salt->context;

  br_hkdf_init(br_hkdf_ctx, &br_sha256_vtable, secret_ctx->key, secret_ctx->key_size);

  if (input_key_material != 0) {
    if (input_key_material->context == 0) {
      error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
      goto exit;
    }

    secret_ctx = (vault_default_secret_key_ctx_t*) input_key_material->context;

    br_hkdf_inject(br_hkdf_ctx, secret_ctx->key, secret_ctx->key_size);
  }

  br_hkdf_flip(br_hkdf_ctx);

  {
    uint8_t                         i          = 0;
    ockam_vault_secret_attributes_t attributes = { .length      = OCKAM_VAULT_SHA256_DIGEST_LENGTH,
                                                   .type        = OCKAM_VAULT_SECRET_TYPE_BUFFER,
                                                   .purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                   .persistence = OCKAM_VAULT_SECRET_EPHEMERAL };

    for (i = 0; i < derived_outputs_count; i++) {
      ockam_vault_secret_t* output = derived_outputs;
      output += i;

      if (output == 0) {
        error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
        goto exit;
      }

      error = vault_default_secret_key_create(vault, output, &attributes, 0, 0, 0);
      if (error != OCKAM_ERROR_NONE) { goto exit; }

      secret_ctx = (vault_default_secret_key_ctx_t*) output->context;

      br_hkdf_produce(br_hkdf_ctx, 0, 0, secret_ctx->key, secret_ctx->key_size);
    }
  }

exit:
  return error;
}

ockam_error_t vault_default_aead_aes_gcm_init(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t                     error            = OCKAM_ERROR_NONE;
  vault_default_aead_aes_gcm_ctx_t* aead_aes_gcm_ctx = 0;

  if ((ctx == 0) || (ctx->memory == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &aead_aes_gcm_ctx, sizeof(vault_default_aead_aes_gcm_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &(aead_aes_gcm_ctx->br_aes_key), sizeof(br_aes_ct_ctr_keys));
  if (error != OCKAM_ERROR_NONE) {
    ockam_memory_free(ctx->memory, aead_aes_gcm_ctx, sizeof(vault_default_aead_aes_gcm_ctx_t));
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(ctx->memory, (void**) &(aead_aes_gcm_ctx->br_aes_gcm_ctx), sizeof(br_gcm_context));
  if (error != OCKAM_ERROR_NONE) {
    ockam_memory_free(ctx->memory, aead_aes_gcm_ctx->br_aes_key, sizeof(br_aes_ct_ctr_keys));
    ockam_memory_free(ctx->memory, aead_aes_gcm_ctx, sizeof(vault_default_aead_aes_gcm_ctx_t));
    goto exit;
  }

  ctx->default_features |= OCKAM_VAULT_FEAT_AEAD_AES_GCM;
  ctx->aead_aes_gcm_ctx = aead_aes_gcm_ctx;

exit:
  return error;
}

ockam_error_t vault_default_aead_aes_gcm_deinit(ockam_vault_shared_context_t* ctx)
{
  ockam_error_t                     error            = OCKAM_ERROR_NONE;
  vault_default_aead_aes_gcm_ctx_t* aead_aes_gcm_ctx = 0;

  if ((ctx == 0) || (ctx->memory == 0) || (ctx->aead_aes_gcm_ctx == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (aead_aes_gcm_ctx->br_aes_gcm_ctx != 0) {
    ockam_memory_free(ctx->memory, aead_aes_gcm_ctx->br_aes_gcm_ctx, sizeof(br_gcm_context));
  }

  if (aead_aes_gcm_ctx->br_aes_key != 0) {
    ockam_memory_free(ctx->memory, aead_aes_gcm_ctx->br_aes_key, sizeof(br_aes_ct_ctr_keys));
  }

  error = ockam_memory_free(ctx->memory, aead_aes_gcm_ctx, sizeof(vault_default_aead_aes_gcm_ctx_t));
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  ctx->default_features &= (!OCKAM_VAULT_FEAT_AEAD_AES_GCM);
  ctx->aead_aes_gcm_ctx = 0;

exit:
  return error;
}

ockam_error_t vault_default_aead_aes_gcm(ockam_vault_t*        vault,
                                         uint8_t               encrypt,
                                         ockam_vault_secret_t* key,
                                         uint16_t              nonce,
                                         const uint8_t*        additional_data,
                                         size_t                additional_data_length,
                                         const uint8_t*        input,
                                         size_t                input_length,
                                         uint8_t*              output,
                                         size_t                output_size,
                                         size_t*               output_length)
{
  ockam_error_t                     error                                  = OCKAM_ERROR_NONE;
  ockam_vault_shared_context_t*     ctx                                    = 0;
  vault_default_secret_key_ctx_t*   secret_ctx                             = 0;
  vault_default_aead_aes_gcm_ctx_t* aead_aes_gcm_ctx                       = 0;
  size_t                            run_length                             = 0;
  uint8_t                           iv[VAULT_DEFAULT_AEAD_AES_GCM_IV_SIZE] = { 0 };

  if ((vault == 0) || (vault->context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  ctx = (ockam_vault_shared_context_t*) vault->context;

  if ((ctx->aead_aes_gcm_ctx == 0) || (!(ctx->default_features & OCKAM_VAULT_FEAT_AEAD_AES_GCM))) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  aead_aes_gcm_ctx = (vault_default_aead_aes_gcm_ctx_t*) ctx->aead_aes_gcm_ctx;

  if ((aead_aes_gcm_ctx->br_aes_key == 0) || (aead_aes_gcm_ctx->br_aes_gcm_ctx == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if (encrypt) {
    if (output_size < input_length + OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }
  }

  if ((key->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) &&
      (key->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (key->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_default_secret_key_ctx_t*) key->context;

  {
    int n = 1;

    if (*(char*) &n == 1) { /* Check the endianness and copy appropriately */
      iv[VAULT_DEFAULT_AEAD_AES_GCM_IV_OFFSET]     = ((nonce >> 8) & 0xFF);
      iv[VAULT_DEFAULT_AEAD_AES_GCM_IV_OFFSET + 1] = ((nonce) &0xFF);
    } else {
      iv[VAULT_DEFAULT_AEAD_AES_GCM_IV_OFFSET]     = ((nonce) &0xFF);
      iv[VAULT_DEFAULT_AEAD_AES_GCM_IV_OFFSET + 1] = ((nonce >> 8) & 0xFF);
    }
  }

  br_aes_ct_ctr_init(aead_aes_gcm_ctx->br_aes_key, secret_ctx->key, secret_ctx->key_size);

  br_gcm_init(aead_aes_gcm_ctx->br_aes_gcm_ctx, &(aead_aes_gcm_ctx->br_aes_key->vtable), br_ghash_ctmul32);

  br_gcm_reset(aead_aes_gcm_ctx->br_aes_gcm_ctx, &iv[0], VAULT_DEFAULT_AEAD_AES_GCM_IV_SIZE);

  br_gcm_aad_inject(aead_aes_gcm_ctx->br_aes_gcm_ctx, additional_data, additional_data_length);

  br_gcm_flip(aead_aes_gcm_ctx->br_aes_gcm_ctx);

  if (encrypt == VAULT_DEFAULT_AEAD_AES_GCM_ENCRYPT) {
    run_length = input_length;
  } else {
    run_length = input_length - OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH;
  }

  ockam_memory_copy(ctx->memory, output, input, run_length);

  br_gcm_run(aead_aes_gcm_ctx->br_aes_gcm_ctx, encrypt, output, run_length);

  if (encrypt == VAULT_DEFAULT_AEAD_AES_GCM_ENCRYPT) {
    uint8_t* tag = output + input_length;
    br_gcm_get_tag(aead_aes_gcm_ctx->br_aes_gcm_ctx, tag);
    *output_length = input_length + OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH;
  } else {
    const uint8_t* tag = input + run_length;
    if (!(br_gcm_check_tag(aead_aes_gcm_ctx->br_aes_gcm_ctx, tag))) {
      error = OCKAM_VAULT_ERROR_INVALID_TAG;
      goto exit;
    }
    *output_length = input_length - OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH;
  }

exit:
  return error;
}

ockam_error_t vault_default_aead_aes_gcm_encrypt(ockam_vault_t*        vault,
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
  return vault_default_aead_aes_gcm(vault,
                                    VAULT_DEFAULT_AEAD_AES_GCM_ENCRYPT,
                                    key,
                                    nonce,
                                    additional_data,
                                    additional_data_length,
                                    plaintext,
                                    plaintext_length,
                                    ciphertext_and_tag,
                                    ciphertext_and_tag_size,
                                    ciphertext_and_tag_length);
}

ockam_error_t vault_default_aead_aes_gcm_decrypt(ockam_vault_t*        vault,
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
  return vault_default_aead_aes_gcm(vault,
                                    VAULT_DEFAULT_AEAD_AES_GCM_DECRYPT,
                                    key,
                                    nonce,
                                    additional_data,
                                    additional_data_length,
                                    ciphertext_and_tag,
                                    ciphertext_and_tag_length,
                                    plaintext,
                                    plaintext_size,
                                    plaintext_length);
}
