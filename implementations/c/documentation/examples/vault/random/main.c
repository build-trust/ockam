/**
 * @file    main.c
 * @brief   Example main file for Vault Random Bytes Generate
 */

#include "ockam/error.h"

#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

#include "ockam/random.h"
#include "ockam/random/urandom.h"

#include "ockam/vault.h"
#include "ockam/vault/default.h"

#include <stdio.h>

/*
 * This example shows how to generate random bytes with a vault.
 *
 * Ockam protocols depend on a variety of standard cryptographic primitives
 * or building blocks. Depending on the environment, these building blocks may
 * be provided by a software implementation or a cryptographically capable
 * hardware component.
 *
 * In order to support a variety of cryptographically capable hardware, we
 * maintain loose coupling between a protocol and how a specific building block
 * is invoked in a specific hardware. This is achieved using the abstract vault
 * interface (defined in `ockam/vault.h`).
 *
 * The default vault is a software-only implementation of the vault interface,
 * which may be used when a particular cryptographic building block is not
 * available in hardware.
 *
 * This example shows how to initialize a handle to the default software vault
 * and use it to generate random bytes with a vault.
 */

int main(void)
{
  int exit_code = 0;

  /*
   * The actions taken below are are covered in the initialization example. For further detail on these
   * actions refer to that example.
   */

  ockam_error_t error        = OCKAM_ERROR_NONE;
  ockam_error_t deinit_error = OCKAM_ERROR_NONE;

  ockam_memory_t                   memory           = { 0 };
  ockam_random_t                   random           = { 0 };
  ockam_vault_t                    vault            = { 0 };
  ockam_vault_default_attributes_t vault_attributes = { .memory = &memory, .random = &random };

  error = ockam_memory_stdlib_init(&memory);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  error = ockam_random_urandom_init(&random);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /*
   * We now have an initialized vault handle of type ockam_vault_t, we can
   * call any of the functions defined in `ockam/vault.h` using this handle.
   *
   * For example we can use it to generate an array of 64 random bytes.
   */

  const size_t random_bytes_length               = 64;
  uint8_t      random_bytes[random_bytes_length] = { 0 };

  error = ockam_vault_random_bytes_generate(&vault, &random_bytes[0], random_bytes_length);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /* Now let's print the random bytes in hexadecimal form. */

  int i;
  for (i = 0; i < random_bytes_length; i++) { printf("%02x", random_bytes[i]); }
  printf("\n");

exit:

  /* Deinitialize to free resources associated with this handle. */

  deinit_error = ockam_vault_deinit(&vault);
  ockam_random_deinit(&random);
  ockam_memory_deinit(&memory);


  if (error == OCKAM_ERROR_NONE) { error = deinit_error; }
  if (error != OCKAM_ERROR_NONE) { exit_code = -1; }
  return exit_code;
}

