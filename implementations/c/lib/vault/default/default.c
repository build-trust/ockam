/**
 ********************************************************************************************************
 * @file    default.c
 * @brief   Interface functions for the default Ockam Vault
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include "default.h"

#include "bearssl.h"
#include "ockam/memory.h"
#include "ockam/vault.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define VAULT_DEFAULT_SHA256_DIGEST_SIZE 32u

#define VAULT_DEFAULT_AES_GCM_DECRYPT 0u
#define VAULT_DEFAULT_AES_GCM_ENCRYPT 1u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_128 16u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_192 24u
#define VAULT_DEFAULT_AES_GCM_KEY_SIZE_256 32u
#define VAULT_DEFAULT_AES_GCM_TAG_SIZE 16u

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

const char *kVaultRandomSeed = "ockam_vault_seed";

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

typedef struct {
  const br_prng_class *br_random;
  void *br_random_ctx;
} VaultDefaultRandomCtx;

typedef struct {
  const br_hash_class *br_sha256;
  void *br_sha256_ctx;
} VaultDefaultSha256Ctx;

typedef struct {
  const br_ec_impl *br_ec;
  uint32_t br_curve;
  br_ec_private_key br_private_key[kMaxOckamVaultKey];
  unsigned char *br_private_key_buf[kMaxOckamVaultKey];
  size_t br_private_key_size;
  size_t br_public_key_size;
} VaultDefaultKeyEcdhCtx;

typedef struct {
  br_aes_ct_ctr_keys *br_aes_keys;
  br_gcm_context *br_aes_gcm_ctx;
} VaultDefaultAesGcmCtx;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

VaultError VaultDefaultRandomCreate(OckamVaultCtx *p_ctx);
VaultError VaultDefaultRandomDestroy(OckamVaultCtx *p_ctx);

VaultError VaultDefaultSha256Create(OckamVaultCtx *p_ctx);
VaultError VaultDefaultSha256Destroy(OckamVaultCtx *p_ctx);

VaultError VaultDefaultKeyEcdhCreate(OckamVaultCtx *p_ctx);
VaultError VaultDefaultKeyEcdhDestroy(OckamVaultCtx *p_ctx);

VaultError VaultDefaultHkdfCreate(OckamVaultCtx *p_ctx);
VaultError VaultDefaultHkdfDestroy(OckamVaultCtx *p_ctx);

VaultError VaultDefaultAesGcmCreate(OckamVaultCtx *p_ctx);
VaultError VaultDefaultAesGcmDestroy(OckamVaultCtx *p_ctx);
VaultError VaultDefaultAesGcm(OckamVaultCtx *p_ctx, int encrypt, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                              size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                              uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

const OckamVault ockam_vault_default = {
    (VaultError(*)(void **, void *, const OckamMemory *)) & VaultDefaultCreate,

    (VaultError(*)(void *)) & VaultDefaultDestroy,

    (VaultError(*)(void *, uint8_t *, size_t)) & VaultDefaultRandom,

    (VaultError(*)(void *, OckamVaultKey)) & VaultDefaultKeyGenerate,

    (VaultError(*)(void *, OckamVaultKey, uint8_t *, size_t)) & VaultDefaultKeyGetPublic,

    (VaultError(*)(void *, OckamVaultKey, uint8_t *, size_t)) & VaultDefaultKeySetPrivate,

    (VaultError(*)(void *, OckamVaultKey, uint8_t *, size_t, uint8_t *, size_t)) & VaultDefaultEcdh,

    (VaultError(*)(void *, uint8_t *, size_t, uint8_t *, size_t)) & VaultDefaultSha256,

    (VaultError(*)(void *, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t)) &
        VaultDefaultHkdf,

    (VaultError(*)(void *, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t, uint8_t *,
                   size_t, uint8_t *, size_t)) &
        VaultDefaultAesGcmEncrypt,

    (VaultError(*)(void *, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t, uint8_t *, size_t, uint8_t *,
                   size_t, uint8_t *, size_t)) &
        VaultDefaultAesGcmDecrypt,
};

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/**
 ********************************************************************************************************
 *                                         VaultDefaultCreate()
 ********************************************************************************************************
 */

VaultError VaultDefaultCreate(OckamVaultCtx **ctx, OckamVaultDefaultConfig *p_cfg, const OckamMemory *memory) {
  VaultError ret_val = kOckamErrorNone;
  OckamVaultCtx *p_ctx = 0;
  uint8_t delete = 0;

  if (p_cfg == 0) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (p_cfg->features == OCKAM_VAULT_ALL) { /* If all features are enabled, this is a standalone  */
    if (memory == 0) {                      /* vault that must be created from scratch. If only   */
      ret_val = kOckamError;                /* some features are enabled, this is a sub-vault and */
      goto exit_block;                      /* the context should already exist.                  */
    }

    ret_val = memory->Alloc((void **)ctx, sizeof(OckamVaultCtx));
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }

    p_ctx = *ctx;
    p_ctx->memory = memory;
    p_ctx->ec = p_cfg->ec;
    p_ctx->default_features = 0;
  } else {
    if (*ctx == 0) {         /* If this is a sub-vault, ensure the context already */
      ret_val = kOckamError; /* exists.                                            */
      goto exit_block;
    }

    p_ctx = *ctx;
  }

  delete = 1;

  if (p_cfg->features & OCKAM_VAULT_RANDOM) {
    ret_val = VaultDefaultRandomCreate(p_ctx);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  if (p_cfg->features & OCKAM_VAULT_SHA256) {
    ret_val = VaultDefaultSha256Create(p_ctx);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  if (p_cfg->features & OCKAM_VAULT_KEY_ECDH) {
    ret_val = VaultDefaultKeyEcdhCreate(p_ctx);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  if (p_cfg->features & OCKAM_VAULT_HKDF) {
    ret_val = VaultDefaultHkdfCreate(p_ctx);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  if (p_cfg->features & OCKAM_VAULT_AES_GCM) {
    ret_val = VaultDefaultAesGcmCreate(p_ctx);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

exit_block:

  if ((ret_val != kOckamErrorNone) && (delete)) { /* If an error occurred during create, delete all     */
    VaultDefaultDestroy(p_ctx);                   /* allocated objects and clear the ctx pointer.       */
    *ctx = 0;
  }

  return ret_val;
}

/**
 ********************************************************************************************************
 *                                         VaultDefaultDestroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultDestroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  const OckamMemory *p_memory = 0;
  uint8_t delete_ctx = 0;

  if (p_ctx == 0) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (p_ctx->default_features & OCKAM_VAULT_ALL) { /* Determine if the context pointer needs to be freed */
    delete_ctx = 1;                                /* before we start disabling features                 */
  }

  if (p_ctx->default_features & OCKAM_VAULT_RANDOM) {
    VaultDefaultRandomDestroy(p_ctx);
  }

  if (p_ctx->default_features & OCKAM_VAULT_SHA256) {
    VaultDefaultSha256Destroy(p_ctx);
  }

  if (p_ctx->default_features & OCKAM_VAULT_KEY_ECDH) {
    VaultDefaultKeyEcdhDestroy(p_ctx);
  }

  if (p_ctx->default_features & OCKAM_VAULT_HKDF) {
    VaultDefaultHkdfDestroy(p_ctx);
  }

  if (p_ctx->default_features & OCKAM_VAULT_AES_GCM) {
    VaultDefaultAesGcmDestroy(p_ctx);
  }

  if (delete_ctx) {
    p_memory = p_ctx->memory;
    p_memory->Free(p_ctx, sizeof(OckamVaultCtx));
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                    VaultDefaultRandomCreate()
 ********************************************************************************************************
 */

VaultError VaultDefaultRandomCreate(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultRandomCtx *p_random_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  ret_val = memory->Alloc((void **)&p_random_ctx, sizeof(VaultDefaultRandomCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_random_ctx->br_random = &br_hmac_drbg_vtable;

  ret_val = memory->Alloc(&(p_random_ctx->br_random_ctx), p_random_ctx->br_random->context_size);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_random_ctx->br_random->init(p_random_ctx->br_random_ctx, &br_sha256_vtable, kVaultRandomSeed,
                                sizeof(kVaultRandomSeed));

  p_ctx->random_ctx = p_random_ctx;
  p_ctx->features |= OCKAM_VAULT_RANDOM;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                    VaultDefaultRandomDestroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultRandomDestroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultRandomCtx *p_random_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx->memory == 0) || (p_ctx->random_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;
  p_random_ctx = p_ctx->random_ctx;

  if (p_random_ctx->br_random_ctx != 0) {
    memory->Free(p_random_ctx->br_random_ctx, p_random_ctx->br_random->context_size);
  }

  ret_val = memory->Free(p_random_ctx, sizeof(VaultDefaultRandomCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features &= (!OCKAM_VAULT_RANDOM);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultRandom()
 ********************************************************************************************************
 */

VaultError VaultDefaultRandom(OckamVaultCtx *p_ctx, uint8_t *p_num, size_t num_size) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultRandomCtx *p_random_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->random_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_RANDOM))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_random_ctx = p_ctx->random_ctx;

  if ((p_random_ctx->br_random == 0) ||     /* Ensure the BearSSL random class and context have   */
      (p_random_ctx->br_random_ctx == 0)) { /* initialized and random is enabled.                 */
    ret_val = kOckamError;
    goto exit_block;
  }

  if (num_size >= 65536) { /* Upper limit for BearSSL random number generation   */
    ret_val = kOckamError; /* TODO Add #define                                   */
    goto exit_block;
  }

  p_random_ctx->br_random->generate(p_random_ctx->br_random_ctx, p_num, num_size);
exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                   VaultDefaultKeyEcdhCreate()
 ********************************************************************************************************
 */

VaultError VaultDefaultKeyEcdhCreate(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  uint8_t i = 0;
  size_t size = 0;
  const OckamMemory *memory = 0;
  VaultDefaultRandomCtx *p_random_ctx = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;
  br_hmac_drbg_context *p_rng = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0) || (p_ctx->random_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;
  p_random_ctx = p_ctx->random_ctx;
  p_rng = p_random_ctx->br_random_ctx;

  ret_val = memory->Alloc((void **)&p_key_ecdh_ctx, sizeof(VaultDefaultKeyEcdhCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
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
      ret_val = kOckamError;
      goto exit_block;
      break;
  }

  size = br_ec_keygen(&(p_rng->vtable),      /* Call keygen without a key structure or buffer to   */
                      p_key_ecdh_ctx->br_ec, /* calculate the size of the private key and allocate */
                      0,                     /* buffers appropriately.                             */
                      0, p_key_ecdh_ctx->br_curve);
  if ((size == 0) || (size > BR_EC_KBUF_PRIV_MAX_SIZE)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_key_ecdh_ctx->br_public_key_size = 0;     /* Public key size to be set by Generate or SetPrivate*/
  p_key_ecdh_ctx->br_private_key_size = size; /* Save the size of the private key                   */

  for (i = 0; i < kMaxOckamVaultKey; i++) {
    ret_val = memory->Alloc((void **)&(p_key_ecdh_ctx->br_private_key_buf[i]), p_key_ecdh_ctx->br_private_key_size);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  p_ctx->key_ecdh_ctx = p_key_ecdh_ctx;
  p_ctx->features |= OCKAM_VAULT_KEY_ECDH;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                   VaultDefaultKeyEcdhDestroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultKeyEcdhDestroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  uint8_t i = 0;
  const OckamMemory *memory = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx->memory == 0) || (p_ctx->key_ecdh_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;
  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  p_ctx->features &= (!OCKAM_VAULT_KEY_ECDH);

  for (i = 0; i < kMaxOckamVaultKey; i++) {
    if (p_key_ecdh_ctx->br_private_key_buf[i] != 0) {
      memory->Free(p_key_ecdh_ctx->br_private_key_buf[i], p_key_ecdh_ctx->br_private_key_size);
    }
  }

  ret_val = memory->Free(p_key_ecdh_ctx, sizeof(VaultDefaultKeyEcdhCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                   VaultDefaultKeyGenerate()
 ********************************************************************************************************
 */

VaultError VaultDefaultKeyGenerate(OckamVaultCtx *p_ctx, OckamVaultKey key_type) {
  VaultError ret_val = kOckamErrorNone;
  size_t size = 0;
  VaultDefaultRandomCtx *p_random_ctx = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;
  br_hmac_drbg_context *p_rng = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (p_ctx->random_ctx == 0) ||
      (!(p_ctx->features & OCKAM_VAULT_RANDOM)) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;
  p_random_ctx = p_ctx->random_ctx;
  p_rng = p_random_ctx->br_random_ctx;

  size = br_ec_keygen(&(p_rng->vtable), p_key_ecdh_ctx->br_ec, &(p_key_ecdh_ctx->br_private_key[key_type]),
                      p_key_ecdh_ctx->br_private_key_buf[key_type], p_key_ecdh_ctx->br_curve);
  if (size == 0) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (p_key_ecdh_ctx->br_public_key_size == 0) {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, 0, &br_private_key);
    if (size == 0) {
      ret_val = kOckamError;
      goto exit_block;
    }

    p_key_ecdh_ctx->br_public_key_size = size;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultKeySetPrivate()
 ********************************************************************************************************
 */

VaultError VaultDefaultKeySetPrivate(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_priv_key,
                                     size_t priv_key_size) {
  VaultError ret_val = kOckamErrorNone;
  size_t size = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if ((p_priv_key == 0) || (priv_key_size != p_key_ecdh_ctx->br_private_key_size)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_ctx->memory->Copy(p_key_ecdh_ctx->br_private_key_buf[key_type], p_priv_key, priv_key_size);

  if (p_key_ecdh_ctx->br_public_key_size == 0) {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, 0, &br_private_key);
    if (size == 0) {
      ret_val = kOckamError;
      goto exit_block;
    }

    p_key_ecdh_ctx->br_public_key_size = size;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultKeyGetPublic()
 ********************************************************************************************************
 */

VaultError VaultDefaultKeyGetPublic(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_pub_key,
                                    size_t pub_key_size) {
  VaultError ret_val = kOckamErrorNone;
  size_t size = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if ((p_key_ecdh_ctx->br_public_key_size == 0) || (p_key_ecdh_ctx->br_private_key_buf[key_type] == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_pub_key == 0) || (pub_key_size != p_key_ecdh_ctx->br_public_key_size)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  {
    const br_ec_private_key br_private_key = {.curve = p_key_ecdh_ctx->br_curve,
                                              .xlen = p_key_ecdh_ctx->br_private_key_size,
                                              .x = p_key_ecdh_ctx->br_private_key_buf[key_type]};

    size = br_ec_compute_pub(p_key_ecdh_ctx->br_ec, 0, p_pub_key, &br_private_key);
    if (size == 0) {
      ret_val = kOckamError;
      goto exit_block;
    }
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultEcdh()
 ********************************************************************************************************
 */

VaultError VaultDefaultEcdh(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size,
                            uint8_t *p_ss, size_t ss_size) {
  VaultError ret_val = kOckamErrorNone;
  size_t xoff = 0;
  size_t xlen = 0;
  uint32_t ret = 0;
  VaultDefaultKeyEcdhCtx *p_key_ecdh_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->key_ecdh_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_KEY_ECDH))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_key_ecdh_ctx = p_ctx->key_ecdh_ctx;

  if (ss_size != p_key_ecdh_ctx->br_private_key_size) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_ctx->memory->Copy(p_ss, p_pub_key, ss_size);

  ret = p_key_ecdh_ctx->br_ec->mul(p_ss, ss_size, p_key_ecdh_ctx->br_private_key_buf[key_type], ss_size,
                                   p_key_ecdh_ctx->br_curve);
  if (ret != 1) {
    ret_val = kOckamError;
    goto exit_block;
  }

  xoff = p_key_ecdh_ctx->br_ec->xoff(p_key_ecdh_ctx->br_curve, &xlen);
  p_ctx->memory->Move(p_ss, p_ss + xoff, xlen);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultSha256Create()
 ********************************************************************************************************
 */

VaultError VaultDefaultSha256Create(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultSha256Ctx *p_sha256_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  ret_val = memory->Alloc((void **)&p_sha256_ctx, sizeof(VaultDefaultSha256Ctx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_sha256_ctx->br_sha256 = &br_sha256_vtable;

  ret_val = memory->Alloc(&(p_sha256_ctx->br_sha256_ctx), p_sha256_ctx->br_sha256->context_size);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features |= OCKAM_VAULT_SHA256;
  p_ctx->sha256_ctx = p_sha256_ctx;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultSha256Destroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultSha256Destroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultSha256Ctx *p_sha256_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx->memory == 0) || (p_ctx->sha256_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;
  p_sha256_ctx = p_ctx->sha256_ctx;

  if (p_sha256_ctx->br_sha256_ctx != 0) {
    memory->Free(p_sha256_ctx->br_sha256_ctx, p_sha256_ctx->br_sha256->context_size);
  }

  ret_val = memory->Free(p_sha256_ctx, sizeof(VaultDefaultSha256Ctx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features &= (!OCKAM_VAULT_SHA256);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultSha256()
 ********************************************************************************************************
 */

VaultError VaultDefaultSha256(OckamVaultCtx *p_ctx, uint8_t *p_msg, size_t msg_size, uint8_t *p_digest,
                              size_t digest_size) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultSha256Ctx *p_sha256_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->sha256_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_SHA256))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_sha256_ctx = p_ctx->sha256_ctx;

  if ((p_sha256_ctx->br_sha256 == 0) || (p_sha256_ctx->br_sha256_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_digest == 0) || (digest_size != VAULT_DEFAULT_SHA256_DIGEST_SIZE)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_sha256_ctx->br_sha256->init(p_sha256_ctx->br_sha256_ctx);
  p_sha256_ctx->br_sha256->update(p_sha256_ctx->br_sha256_ctx, p_msg, msg_size);
  p_sha256_ctx->br_sha256->out(p_sha256_ctx->br_sha256_ctx, p_digest);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                    VaultDefaultHkdfCreate()
 ********************************************************************************************************
 */

VaultError VaultDefaultHkdfCreate(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  ret_val = memory->Alloc(&(p_ctx->hkdf_ctx), sizeof(br_hkdf_context));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features |= OCKAM_VAULT_HKDF;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultHkdfDestroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultHkdfDestroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  const OckamMemory *memory = 0;

  if ((p_ctx->memory == 0) || (p_ctx->hkdf_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  ret_val = memory->Free(p_ctx->hkdf_ctx, sizeof(br_hkdf_context));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features &= (!OCKAM_VAULT_HKDF);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultDefaultHkdf()
 ********************************************************************************************************
 */

VaultError VaultDefaultHkdf(OckamVaultCtx *p_ctx, uint8_t *p_salt, size_t salt_size, uint8_t *p_ikm, size_t ikm_size,
                            uint8_t *p_info, size_t info_size, uint8_t *p_out, size_t out_size) {
  VaultError ret_val = kOckamErrorNone;
  br_hkdf_context *p_hkdf_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->hkdf_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_HKDF))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_out == 0) || (out_size == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_ikm == 0) != (ikm_size == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_ctx->memory->Set(p_hkdf_ctx, 0, sizeof(br_hkdf_context));

  br_hkdf_init(p_ctx->hkdf_ctx, /* TODO: Absent salt?                                 */
               &br_sha256_vtable, p_salt, salt_size);

  br_hkdf_inject(p_ctx->hkdf_ctx, p_ikm, ikm_size);

  br_hkdf_flip(p_ctx->hkdf_ctx);

  br_hkdf_produce(p_ctx->hkdf_ctx, p_info, info_size, p_out, out_size);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultAesGcmCreate()
 ********************************************************************************************************
 */

VaultError VaultDefaultAesGcmCreate(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultAesGcmCtx *p_aes_gcm_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  ret_val = memory->Alloc((void **)&p_aes_gcm_ctx, sizeof(VaultDefaultAesGcmCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = memory->Alloc((void **)&(p_aes_gcm_ctx->br_aes_keys), sizeof(br_aes_ct_ctr_keys));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = memory->Alloc((void **)&(p_aes_gcm_ctx->br_aes_gcm_ctx), sizeof(br_gcm_context));
  if (ret_val != kOckamErrorNone) {
    memory->Free(p_aes_gcm_ctx->br_aes_keys, sizeof(br_aes_ct_ctr_keys));
    goto exit_block;
  }

  p_ctx->features |= OCKAM_VAULT_AES_GCM;
  p_ctx->aes_gcm_ctx = p_aes_gcm_ctx;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultAesGcmDestroy()
 ********************************************************************************************************
 */

VaultError VaultDefaultAesGcmDestroy(OckamVaultCtx *p_ctx) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultAesGcmCtx *p_aes_gcm_ctx = 0;
  const OckamMemory *memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0) || (p_ctx->aes_gcm_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  memory = p_ctx->memory;

  if (p_aes_gcm_ctx->br_aes_gcm_ctx != 0) {
    memory->Free(p_aes_gcm_ctx->br_aes_gcm_ctx, sizeof(br_gcm_context));
  }

  if (p_aes_gcm_ctx->br_aes_keys != 0) {
    memory->Free(p_aes_gcm_ctx->br_aes_keys, sizeof(br_aes_ct_ctr_keys));
  }

  ret_val = memory->Free(p_aes_gcm_ctx, sizeof(VaultDefaultAesGcmCtx));
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_ctx->features &= (!OCKAM_VAULT_AES_GCM);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                     VaultDefaultAesGcm()
 ********************************************************************************************************
 */

VaultError VaultDefaultAesGcm(OckamVaultCtx *p_ctx, int encrypt, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                              size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                              uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  VaultError ret_val = kOckamErrorNone;
  VaultDefaultAesGcmCtx *p_aes_gcm_ctx = 0;

  if ((p_ctx == 0) || (p_ctx->aes_gcm_ctx == 0) || (!(p_ctx->features & OCKAM_VAULT_AES_GCM))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_aes_gcm_ctx = p_ctx->aes_gcm_ctx;

  if ((p_aes_gcm_ctx->br_aes_keys == 0) || (p_aes_gcm_ctx->br_aes_gcm_ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_key == 0) ||
      ((key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_128) && (key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_192) &&
       (key_size != VAULT_DEFAULT_AES_GCM_KEY_SIZE_256))) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (tag_size != VAULT_DEFAULT_AES_GCM_TAG_SIZE) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_input == 0) != (input_size == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_output == 0) != (output_size == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (input_size != output_size) {
    ret_val = kOckamError;
    goto exit_block;
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
      ret_val = kOckamError;
      goto exit_block;
    }
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultAesGcmDecrypt()
 ********************************************************************************************************
 */

VaultError VaultDefaultAesGcmEncrypt(OckamVaultCtx *p_ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                     size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                     uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return VaultDefaultAesGcm(p_ctx, VAULT_DEFAULT_AES_GCM_ENCRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size,
                            p_tag, tag_size, p_input, input_size, p_output, output_size);
}

/**
 ********************************************************************************************************
 *                                  VaultDefaultAesGcmDecrypt()
 ********************************************************************************************************
 */

VaultError VaultDefaultAesGcmDecrypt(OckamVaultCtx *p_ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                     size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                     uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return VaultDefaultAesGcm(p_ctx, VAULT_DEFAULT_AES_GCM_DECRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size,
                            p_tag, tag_size, p_input, input_size, p_output, output_size);
}
