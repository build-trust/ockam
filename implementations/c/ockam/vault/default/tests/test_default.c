/**
 * @file        test_default.c
 * @brief
 */

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/random.h"
#include "ockam/vault.h"

#include "ockam/memory/stdlib.h"
#include "ockam/random/urandom.h"
#include "ockam/vault/default.h"

#include "cmocka.h"
#include "test_vault.h"

/**
 * @brief   Main point of entry for default vault test
 */
int main(void)
{
  int                              rc               = 0;
  ockam_vault_t                    vault            = { 0 };
  ockam_memory_t                   memory           = { 0 };
  ockam_random_t                   random           = { 0 };
  ockam_vault_default_attributes_t vault_attributes = { .memory = &memory, .random = &random };

  cmocka_set_message_output(CM_OUTPUT_XML);

  ockam_error_t error = ockam_memory_stdlib_init(&memory);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Memory\r\n");
    goto exit;
  }

  error = ockam_random_urandom_init(&random);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Random\r\n");
    goto exit;
  }

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Vault\r\n");
    goto exit;
  }

  test_vault_run_random(&vault, &memory);
  test_vault_run_sha256(&vault, &memory);
  test_vault_run_secret_ecdh(&vault, &memory, OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY, 1);
  test_vault_run_hkdf(&vault, &memory);
  test_vault_run_aead_aes_gcm(&vault, &memory, TEST_VAULT_AEAD_AES_GCM_KEY_BOTH);

exit:
  if (ockam_error_has_error(&error)) { rc = -1; }

  return rc;
}
