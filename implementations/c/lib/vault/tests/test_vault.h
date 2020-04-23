/**
 * @file    test_vault.h
 * @brief   Common test functions for vault
 */

#ifndef TEST_VAULT_H_
#define TEST_VAULT_H_

#include <stdint.h>

#include "ockam/vault.h"

#define TEST_VAULT_NO_TEST_CASE 0xFF

int test_vault_run_random(ockam_vault_t* vault, ockam_memory_t* memory);
int test_vault_run_sha256(ockam_vault_t* vault, ockam_memory_t* memory);

#if 0
int test_vault_run_key_ecdh(const OckamVault *p_vault, void *p_vault_ctx, const OckamMemory *p_memory, OckamVaultEc ec,
                            uint8_t load_keys);
int test_vault_run_hkdf(const OckamVault *p_vault, void *p_vault_ctx, const OckamMemory *p_memory);
int test_vault_run_aes_gcm(const OckamVault *p_vault, void *p_vault_ctx, const OckamMemory *p_memory);
#endif

#endif
