/**
 * @file    main.c
 * @brief   Example main file for Vault HKDF-SHA256
 */

#include "ockam/error.h"

#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

#include "ockam/random.h"
#include "ockam/random/urandom.h"

#include "ockam/vault.h"
#include "ockam/vault/default.h"

#include <stdio.h>
#include <string.h>


#define EXAMPLE_VAULT_HKDF_IKM_LENGTH  32u
#define EXAMPLE_VAULT_HKDF_SALT_LENGTH 28u

uint8_t g_hkdf_ikm[] =
{
  0x37, 0xe0, 0xe7, 0xda, 0xac, 0xbd, 0x6b, 0xfb,
  0xf6, 0x69, 0xa8, 0x46, 0x19, 0x6f, 0xd4, 0x4d,
  0x1c, 0x87, 0x45, 0xd3, 0x3f, 0x2b, 0xe4, 0x2e,
  0x31, 0xd4, 0x67, 0x41, 0x99, 0xad, 0x00, 0x5e
};

uint8_t g_hkdf_salt[] =
{
  0x4e, 0x6f, 0x69, 0x73, 0x65, 0x5f, 0x58, 0x58,
  0x5f, 0x32, 0x35, 0x35, 0x31, 0x39, 0x5f, 0x41,
  0x45, 0x53, 0x47, 0x43, 0x4d, 0x5f, 0x53, 0x48,
  0x41, 0x32, 0x35, 0x36
};

/*
 * This example shows how to generate derived outputs with HKDF-SHA256.
 *
 * It demonstrates how to use the default software implementation
 * of the Ockam vault interface (defined in `ockam/vault.h`) for
 * HMAC-based Derived Key Function using SHA-256 (HKDF-SHA256).
 *
 * The HKDF operation used in this example is `HKDF-SHA256`, which is
 * defined in [RFC 5869](https://tools.ietf.org/html/rfc5869).
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
   * The HKDF-SHA256 operation requires a salt secret, and while not required, typically also takes
   * in an input key material secret. Since HKDF-SHA256 needs this data as secrets, the data must be
   * loaded into a secret before the HKDF-SHA256 operation can be called. The example code below
   * shows loading salt and input key material data into two secret types.
   */

  ockam_vault_secret_t            salt               = { 0 };
  ockam_vault_secret_t            input_key_material = { 0 };
  ockam_vault_secret_attributes_t attributes         = { 0 };

  attributes.type        = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;

  attributes.length = EXAMPLE_VAULT_HKDF_IKM_LENGTH;
  error = ockam_vault_secret_import(&vault,
                                    &input_key_material,
                                    &attributes,
                                    &g_hkdf_ikm[0],
                                    EXAMPLE_VAULT_HKDF_IKM_LENGTH);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  attributes.length = EXAMPLE_VAULT_HKDF_SALT_LENGTH;
  error = ockam_vault_secret_import(&vault,
                                    &salt,
                                    &attributes,
                                    &g_hkdf_salt[0],
                                    EXAMPLE_VAULT_HKDF_SALT_LENGTH);
  if (error != OCKAM_ERROR_NONE) { goto exit; }


  /*
   * Once the salt and input key material secrets are loaded, the HKDF-SHA256 function can be called.
   * The output of the function is an array of 32-byte derived outputs stored in secret types. The
   * number of derived outputs depends on the number specified in the HKDF-SHA256 call. Typically the
   * number of derived outputs will be 2 or 3.
   */

  ockam_vault_secret_t derived_outputs[2];

  error = ockam_vault_hkdf_sha256(&vault,
                                  &salt,
                                  &input_key_material,
                                  2,
                                  &derived_outputs[0]);
  if (error != OCKAM_ERROR_NONE) { goto exit; }

  size_t  derived_output_length = 0;
  uint8_t derived_output_data_0[OCKAM_VAULT_SHA256_DIGEST_LENGTH] = { 0 };
  uint8_t derived_output_data_1[OCKAM_VAULT_SHA256_DIGEST_LENGTH] = { 0 };

  error = ockam_vault_secret_export(&vault,
                                    &derived_outputs[0],
                                    &derived_output_data_0[0],
                                    OCKAM_VAULT_SHA256_DIGEST_LENGTH,
                                    &derived_output_length);
  if (error != OCKAM_ERROR_NONE) { goto exit; }
  if (derived_output_length != OCKAM_VAULT_SHA256_DIGEST_LENGTH) { goto exit; }

  error = ockam_vault_secret_export(&vault,
                                    &derived_outputs[1],
                                    &derived_output_data_1[0],
                                    OCKAM_VAULT_SHA256_DIGEST_LENGTH,
                                    &derived_output_length);
  if (error != OCKAM_ERROR_NONE) { goto exit; }
  if (derived_output_length != OCKAM_VAULT_SHA256_DIGEST_LENGTH) { goto exit; }


  int i;
  printf("Derived Output 0: ");
  for (i = 0; i < OCKAM_VAULT_SHA256_DIGEST_LENGTH; i++) { printf("%02x", derived_output_data_0[i]); }
  printf("\n");

  printf("Derived Output 1: ");
  for (i = 0; i < OCKAM_VAULT_SHA256_DIGEST_LENGTH; i++) { printf("%02x", derived_output_data_1[i]); }
  printf("\n");

exit:

  /* Destroy the secrets to free the associated resources */

  ockam_vault_secret_destroy(&vault, &salt);
  ockam_vault_secret_destroy(&vault, &input_key_material);
  ockam_vault_secret_destroy(&vault, &derived_outputs[0]);
  ockam_vault_secret_destroy(&vault, &derived_outputs[1]);

  /* Deinitialize to free resources associated with this handle. Save the vault deinit error status.*/

  deinit_error = ockam_vault_deinit(&vault);
  ockam_random_deinit(&random);
  ockam_memory_deinit(&memory);

  if (error == OCKAM_ERROR_NONE) { error = deinit_error; }
  if (error != OCKAM_ERROR_NONE) { exit_code = -1; }
  return exit_code;
}

