/**
 * @file    atecc608a.h
 * @brief   Vault interface for Microchip ATECC608A
 */

#ifndef ATECC608A_H_
#define ATECC608A_H_

#include "ockam/vault.h"
#include "ockam/memory.h"
#include "ockam/mutex.h"

#include "ockam/vault/impl.h"

#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"
#include "basic/atca_basic_aes_gcm.h"

extern const char* const OCKAM_VAULT_ATECC608A_ERROR_DOMAIN;

typedef enum {
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_PARAM             = 1,
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_ATTRIBUTES        = 2,
  OCKAM_VAULT_ATECC608A_ERROR_INIT_FAIL                 = 3,
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_CONTEXT           = 4,
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_SIZE              = 5,
  OCKAM_VAULT_ATECC608A_ERROR_RANDOM_FAIL               = 6,
  OCKAM_VAULT_ATECC608A_ERROR_SHA256_FAIL               = 7,
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_SECRET            = 8,
  OCKAM_VAULT_ATECC608A_ERROR_SECRET_GENERATE_FAIL      = 9,
  OCKAM_VAULT_ATECC608A_ERROR_SECRET_IMPORT_FAIL        = 10,
  OCKAM_VAULT_ATECC608A_ERROR_ECDH_FAIL                 = 11,
  OCKAM_VAULT_ATECC608A_ERROR_PUBLIC_KEY_FAIL           = 12,
  OCKAM_VAULT_ATECC608A_ERROR_INVALID_SECRET_TYPE       = 13,
  OCKAM_VAULT_ATECC608A_ERROR_HKDF_SHA256_FAIL          = 14,
  OCKAM_VAULT_ATECC608A_ERROR_AEAD_AES_GCM_FAIL         = 15,
} ockam_error_code_vault_atecc608a_t;

#define OCKAM_VAULT_ATECC608A_IO_PROTECTION_KEY_SIZE  32u

typedef struct {
  uint8_t key[OCKAM_VAULT_ATECC608A_IO_PROTECTION_KEY_SIZE];
  uint8_t key_size;
  uint8_t slot;
} ockam_vault_atecc608a_io_protection_t;

typedef struct
{
  ockam_memory_t *                       memory;
  ockam_mutex_t*                         mutex;
  ATCAIfaceCfg*                          atca_iface_cfg;
  ockam_vault_atecc608a_io_protection_t* io_protection;
} ockam_vault_atecc608a_attributes_t;

ockam_error_t ockam_vault_atecc608a_init(ockam_vault_t* vault, ockam_vault_atecc608a_attributes_t* attributes);

#endif
