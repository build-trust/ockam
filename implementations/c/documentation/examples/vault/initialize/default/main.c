/**
 * @file    main.c
 * @brief   Example main file for Default Vault Initialize.
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
 * This example shows how to initialize a handle to the default vault.
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
 * and use it call vault interface functions.
 */

int main(void)
{
  int exit_code = 0;

  /*
   * All ockam functions return `ockam_error_t`. A function was successful
   * if the return value, `error == OCKAM_ERROR_NONE`
   *
   * This variable is used below to store and check function return values.
   */
  ockam_error_t error        = OCKAM_ERROR_NONE;
  ockam_error_t deinit_error = OCKAM_ERROR_NONE;

  /*
   * Before we can initialize a handle to the default vault, we must first
   * initialize a handle to an implementation of the memory interface, which
   * is defined in `ockam/memory.h`.
   *
   * The default vault requires a memory implementation handle at
   * initialization. This approach allows us to plugin the strategy for where
   * and how a vault should allocate memory.
   *
   * We may provide a memory implementation that allocates using
   * stdlib (malloc, free ...) or we may instead provide a an implementation
   * that allocates from a fixed sized buffer.
   *
   * In this example we use the stdlib implementation of the memory interface.
   */

  ockam_memory_t memory;

  error = ockam_memory_stdlib_init(&memory);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /*
   * Another component needed by the default vault is a random generator
   * to generate a seed to initialize the default vault's psuedorandom
   * generator. The random generator needs to be initialized before the
   * default vault is initialized because the random generator must be passed
   * in as one of the initialization attributes.
   *
   * In this example we use the urandom implementation of the random interface.
   */

  ockam_random_t random;
  error = ockam_random_urandom_init(&random);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /*
   * To initialize a handle to the default vault, we define a variable of the
   * generic type `ockam_vault_t` that will hold a handle to our vault.
   *
   * We also set the initialization attributes in a struct of
   * type `ockam_vault_default_attributes_t`
   *
   * We then pass the address of both these variable to the default
   * implementation specific initialization function.
   */

  ockam_vault_t                    vault;
  ockam_vault_default_attributes_t vault_attributes = { .memory = &memory, .random = &random };

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /*
   * We now have an initialized vault handle of type ockam_vault_t, we can
   * call any of the functions defined in `ockam/vault.h` using this handle.
   *
   * For example we can use it to generate an array of 32 random bytes.
   */

  const size_t random_bytes_length               = 32;
  uint8_t      random_bytes[random_bytes_length] = { 0 };

  error = ockam_vault_random_bytes_generate(&vault, &random_bytes[0], random_bytes_length);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  /* Now let's print the random bytes in hexadecimal form. */

  int i;
  for (i = 0; i < random_bytes_length; i++) { printf("%02x", random_bytes[i]); }
  printf("\n");

exit:

  /* Deinitialize to free resources associated with this handle. Save the vault deinit error status.*/
  deinit_error = ockam_vault_deinit(&vault);
  ockam_random_deinit(&random);
  ockam_memory_deinit(&memory);

  /*
   * If we reached the exit label without any errors, everything ran fine. At this point we can use the
   * result of the vault deinit as the last error check for the return status. If there was an error
   * that triggered a goto exit, we would not want to overwrite that error with the deinit status. In this
   * example its not as important since we just set an error code, but in situations where we want to log
   * the error code, its important to not overwrite the error with the deinit status.
   */

  if (error == OCKAM_ERROR_NONE) { error = deinit_error; }
  if (error != OCKAM_ERROR_NONE) { exit_code = -1; }
  return exit_code;
}

