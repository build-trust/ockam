/**
 * @file    atecc608a.h
 * @brief   Vault interface for Microchip ATECC608A
 */

#ifndef ATECC608A_H_
#define ATECC608A_H_

#include "ockam/vault.h"
#include "ockam/memory.h"
#include "ockam/mutex.h"

#include "vault/impl.h"

#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"
#include "basic/atca_basic_aes_gcm.h"

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
