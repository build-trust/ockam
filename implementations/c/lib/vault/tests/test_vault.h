/**
 * @file    test_vault.h
 * @brief   Common test functions for vault
 */

#ifndef TEST_VAULT_H_
#define TEST_VAULT_H_

#include <stdint.h>

#include "ockam/vault.h"

#define TEST_VAULT_NO_TEST_CASE 0xFF

typedef enum {
  TEST_VAULT_AEAD_AES_GCM_KEY_128_ONLY = 0x00,
  TEST_VAULT_AEAD_AES_GCM_KEY_256_ONLY,
  TEST_VAULT_AEAD_AES_GCM_KEY_BOTH
} TEST_VAULT_AEAD_AES_GCM_KEY_e;

int test_vault_run_random(ockam_vault_t*  vault,
                          ockam_memory_t* memory);

int test_vault_run_sha256(ockam_vault_t*  vault,
                          ockam_memory_t* memory);

int test_vault_run_secret_ecdh(ockam_vault_t*            vault,
                               ockam_memory_t*           memory,
                               ockam_vault_secret_type_t type,
                               uint8_t                   load_keys);

int test_vault_run_hkdf(ockam_vault_t*  vault,
                        ockam_memory_t* memory);

int test_vault_run_aead_aes_gcm(ockam_vault_t*                vault,
                                ockam_memory_t*               memory,
                                TEST_VAULT_AEAD_AES_GCM_KEY_e key);

#endif
