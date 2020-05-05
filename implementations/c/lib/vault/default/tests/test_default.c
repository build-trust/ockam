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
#include "ockam/vault.h"

#include "memory/stdlib/stdlib.h"
#include "vault/default/default.h"

#include "cmocka.h"
#include "test_vault.h"

/**
 * @brief   Main point of entry for default vault test
 */
int main(void)
{
  int                              rc               = 0;
  ockam_error_t                    error            = OCKAM_ERROR_NONE;
  ockam_vault_t                    vault            = { 0 };
  ockam_memory_t                   memory           = { 0 };
  ockam_vault_default_attributes_t vault_attributes = { .memory = &memory };

  cmocka_set_message_output(CM_OUTPUT_XML);

  error = ockam_memory_stdlib_init(&memory);
  if (error != OCKAM_ERROR_NONE) {
    printf("FAIL: Memory\r\n");
    goto exit;
  }

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (error != OCKAM_ERROR_NONE) {
    printf("FAIL: Vault\r\n");
    goto exit;
  }

  test_vault_run_random(&vault, &memory);
  test_vault_run_sha256(&vault, &memory);
  test_vault_run_secret_ecdh(&vault, &memory, OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY, 1);
  test_vault_run_hkdf(&vault, &memory);
  test_vault_run_aead_aes_gcm(&vault, &memory);

exit:
  if (error != OCKAM_ERROR_NONE) { rc = -1; }

  return rc;
}
