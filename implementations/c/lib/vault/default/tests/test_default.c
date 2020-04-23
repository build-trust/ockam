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
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  test_vault_run_random(&vault, &memory);
  test_vault_run_sha256(&vault, &memory);

#if 0
  TestVaultRunKeyEcdh(vault, default_0, memory, default_cfg.ec, 1);
  TestVaultRunHkdf(vault, default_0, memory);
  TestVaultRunAesGcm(vault, default_0, memory);
#endif

exit:
  if (error != OCKAM_ERROR_NONE) { rc = -1; }

  return rc;
}
