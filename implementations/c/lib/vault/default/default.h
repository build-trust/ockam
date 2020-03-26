/**
 ********************************************************************************************************
 * @file    default.h
 * @brief
 ********************************************************************************************************
 */

#ifndef DEFAULT_H_
#define DEFAULT_H_

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include "ockam/memory.h"
#include "ockam/vault.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @struct  OckamVaultDefaultConfig
 * @brief
 *******************************************************************************
 */

typedef struct {
  uint32_t features;
  OckamVaultEc ec;
} OckamVaultDefaultConfig;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

OckamError VaultDefaultCreate(OckamVaultCtx **ctx, OckamVaultDefaultConfig *p_cfg, const OckamMemory *memory);

OckamError VaultDefaultDestroy(OckamVaultCtx *ctx);

OckamError VaultDefaultRandom(OckamVaultCtx *ctx, uint8_t *p_num, size_t num_size);

OckamError VaultDefaultKeyGenerate(OckamVaultCtx *ctx, OckamVaultKey key_type);

OckamError VaultDefaultKeyGetPublic(OckamVaultCtx *ctx, OckamVaultKey key_type, uint8_t *p_pub_key,
                                    size_t pub_key_size);

OckamError VaultDefaultKeySetPrivate(OckamVaultCtx *ctx, OckamVaultKey key_type, uint8_t *p_priv_key,
                                     size_t priv_key_size);

OckamError VaultDefaultEcdh(OckamVaultCtx *ctx, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size,
                            uint8_t *p_ss, size_t ss_size);

OckamError VaultDefaultSha256(OckamVaultCtx *ctx, uint8_t *p_msg, size_t msg_size, uint8_t *p_digest,
                              size_t digest_size);

OckamError VaultDefaultHkdf(OckamVaultCtx *ctx, uint8_t *p_salt, size_t salt_size, uint8_t *p_ikm, size_t ikm_size,
                            uint8_t *p_info, size_t info_size, uint8_t *p_out, size_t out_size);

OckamError VaultDefaultAesGcmEncrypt(OckamVaultCtx *ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv, size_t iv_size,
                                     uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size, uint8_t *p_input,
                                     size_t input_size, uint8_t *p_output, size_t output_size);

OckamError VaultDefaultAesGcmDecrypt(OckamVaultCtx *ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv, size_t iv_size,
                                     uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size, uint8_t *p_input,
                                     size_t input_size, uint8_t *p_output, size_t output_size);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

extern const OckamVault ockam_vault_default;

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

#endif
