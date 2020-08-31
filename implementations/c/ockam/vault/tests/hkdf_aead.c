/**
 * @file    hkdf_aead.c
 * @brief   Common HKDF & AEAD AES GCM test functions for Ockam Vault
 */

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>

#include "cmocka.h"
#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/vault.h"
#include "test_vault.h"

#define TEST_VAULT_HKDF_AEAD_TEST_CASES          1u
#define TEST_VAULT_HKDF_AEAD_NAME_SIZE           32u
#define TEST_VAULT_HKDF_AEAD_TAG_SIZE            16u

/**
 * @struct  test_vault_hkdf_aead_data_t
 * @brief
 */
typedef struct {
  uint8_t* salt;
  uint32_t salt_size;
  uint8_t* ikm;
  uint32_t ikm_size;
  uint8_t* aad;
  uint32_t aad_size;
  uint16_t nonce;
  uint8_t* plaintext;
  uint8_t* ciphertext_and_tag;
  uint32_t text_size;
} test_vault_hkdf_aead_data_t;

/**
 * @struct  test_vault_hkdf_aead_shared_data_t
 * @brief   Shared test data for all unit tests
 */
typedef struct {
  uint16_t        test_count;
  uint16_t        test_count_max;
  ockam_vault_t*  vault;
  ockam_memory_t* memory;
} test_vault_hkdf_aead_shared_data_t;

void test_vault_hkdf_aead(void** state);
int  test_vault_hkdf_aead_teardown(void** state);

/* clang-format off */

uint8_t g_hkdf_aead_test_ikm[] =
{
  0x37, 0xe0, 0xe7, 0xda, 0xac, 0xbd, 0x6b, 0xfb,
  0xf6, 0x69, 0xa8, 0x46, 0x19, 0x6f, 0xd4, 0x4d,
  0x1c, 0x87, 0x45, 0xd3, 0x3f, 0x2b, 0xe4, 0x2e,
  0x31, 0xd4, 0x67, 0x41, 0x99, 0xad, 0x00, 0x5e
};

uint8_t g_hkdf_aead_test_salt[] =
{
  0x4e, 0x6f, 0x69, 0x73, 0x65, 0x5f, 0x58, 0x58,
  0x5f, 0x32, 0x35, 0x35, 0x31, 0x39, 0x5f, 0x41,
  0x45, 0x53, 0x47, 0x43, 0x4d, 0x5f, 0x53, 0x48,
  0x41, 0x32, 0x35, 0x36
};

uint8_t g_hkdf_aead_test_aad[] =
{
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xab, 0xad, 0xda, 0xd2
};

uint8_t g_hkdf_aead_test_plaintext[] =
{
  0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
  0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F
};

uint8_t g_hkdf_aead_test_ciphertext_and_tag[] =
{
  0x84, 0x4f, 0x7c, 0x13, 0x2f, 0xac, 0xdb, 0x60,
  0x00, 0x0f, 0xe2, 0x5d, 0x1e, 0x66, 0xb1, 0x35,
  0xab, 0xec, 0x4b, 0x72, 0x99, 0x52, 0x0f, 0x5e,
  0xfb, 0x18, 0xd1, 0xe6, 0x36, 0xf7, 0x3f, 0xc4,
};

test_vault_hkdf_aead_data_t g_hkdf_aead_data[TEST_VAULT_HKDF_AEAD_TEST_CASES] =
{
  {
    &g_hkdf_aead_test_salt[0],
    28,
    &g_hkdf_aead_test_ikm[0],
    32,
    &g_hkdf_aead_test_aad[0],
    20,
    0xCAFE,
    &g_hkdf_aead_test_plaintext[0],
    &g_hkdf_aead_test_ciphertext_and_tag[0],
    16
  }
};

/* clang-format on */

/**
 * @brief   Common test functions for HKDF and AEAD AES GCM using Ockam Vault
 */

void test_vault_hkdf_aead(void** state)
{
  uint8_t                             i                           = 0;
  size_t                              length                      = 0;
  size_t                              hkdf_aead_encrypt_hash_size = 0;
  size_t                              hkdf_aead_decrypt_data_size = 0;
  uint8_t*                            hkdf_aead_encrypt_hash      = 0;
  uint8_t*                            hkdf_aead_decrypt_data      = 0;
  test_vault_hkdf_aead_shared_data_t* test_data                   = 0;

  ockam_vault_secret_t            ikm_secret  = { 0 };
  ockam_vault_secret_t            salt_secret = { 0 };
  ockam_vault_secret_attributes_t attributes  = { 0 };

  ockam_vault_secret_t hkdf_aes_key = { 0 };

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  test_data = (test_vault_hkdf_aead_shared_data_t*) *state;

  if (test_data->test_count >= test_data->test_count_max) {
    fail_msg("Test count %d has exceeded max test count of %d", test_data->test_count, test_data->test_count_max);
  }

  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  attributes.type        = OCKAM_VAULT_SECRET_TYPE_BUFFER;

  hkdf_aead_encrypt_hash_size = g_hkdf_aead_data[test_data->test_count].text_size + TEST_VAULT_HKDF_AEAD_TAG_SIZE;

  ockam_error_t error = ockam_memory_alloc_zeroed(test_data->memory,
                                    (void**) &hkdf_aead_encrypt_hash,
                                    hkdf_aead_encrypt_hash_size);
  if (ockam_error_has_error(&error)) {
    fail_msg("Unable to alloc hkdf_aead_encrypt_hash");
  }

  hkdf_aead_decrypt_data_size = g_hkdf_aead_data[test_data->test_count].text_size;

  error =
    ockam_memory_alloc_zeroed(test_data->memory, (void**) &hkdf_aead_decrypt_data, hkdf_aead_decrypt_data_size);
  if (ockam_error_has_error(&error)) {
    fail_msg("Unable to alloc hkdf_aead_decrypt_data");
  }

  attributes.length = g_hkdf_aead_data[test_data->test_count].salt_size;
  error             = ockam_vault_secret_import(test_data->vault,
                                                &salt_secret,
                                                &attributes,
                                                g_hkdf_aead_data[test_data->test_count].salt,
                                                g_hkdf_aead_data[test_data->test_count].salt_size);
  assert_true(ockam_error_is_none(&error));

  attributes.length = g_hkdf_aead_data[test_data->test_count].ikm_size;
  error             = ockam_vault_secret_import(test_data->vault,
                                                &ikm_secret,
                                                &attributes,
                                                g_hkdf_aead_data[test_data->test_count].ikm,
                                                g_hkdf_aead_data[test_data->test_count].ikm_size);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_hkdf_sha256(test_data->vault,
                                  &salt_secret,
                                  &ikm_secret,
                                  1,
                                  &hkdf_aes_key);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_type_set(test_data->vault,
                                      &hkdf_aes_key,
                                      OCKAM_VAULT_SECRET_TYPE_AES128_KEY);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_aead_aes_gcm_encrypt(test_data->vault,
                                           &hkdf_aes_key,
                                           g_hkdf_aead_data[test_data->test_count].nonce,
                                           g_hkdf_aead_data[test_data->test_count].aad,
                                           g_hkdf_aead_data[test_data->test_count].aad_size,
                                           g_hkdf_aead_data[test_data->test_count].plaintext,
                                           g_hkdf_aead_data[test_data->test_count].text_size,
                                           hkdf_aead_encrypt_hash,
                                           hkdf_aead_encrypt_hash_size,
                                           &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, hkdf_aead_encrypt_hash_size);

  assert_memory_equal(hkdf_aead_encrypt_hash,
                      g_hkdf_aead_data[test_data->test_count].ciphertext_and_tag,
                      hkdf_aead_encrypt_hash_size);

  error = ockam_vault_aead_aes_gcm_decrypt(test_data->vault,
                                           &hkdf_aes_key,
                                           g_hkdf_aead_data[test_data->test_count].nonce,
                                           g_hkdf_aead_data[test_data->test_count].aad,
                                           g_hkdf_aead_data[test_data->test_count].aad_size,
                                           g_hkdf_aead_data[test_data->test_count].ciphertext_and_tag,
                                           hkdf_aead_encrypt_hash_size,
                                           hkdf_aead_decrypt_data,
                                           hkdf_aead_decrypt_data_size,
                                           &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, hkdf_aead_decrypt_data_size);

  assert_memory_equal(
    hkdf_aead_decrypt_data, g_hkdf_aead_data[test_data->test_count].plaintext, hkdf_aead_decrypt_data_size);
}

/**
 * @brief   Common unit test teardown function for HKDF using Ockam Vault
 * @param   state   Contains a pointer to shared data for all HKDF test cases.
 */

int test_vault_hkdf_aead_teardown(void** state)
{
  test_vault_hkdf_aead_shared_data_t* test_data = 0;

  /* ------------------- */
  /* Test Case Increment */
  /* ------------------- */

  test_data = (test_vault_hkdf_aead_shared_data_t*) *state;
  test_data->test_count++;

  return 0;
}

/**
 * @brief   Triggers HKDF unit tests using Ockam Vault.
 * @return  Zero on success. Non-zero on failure.
 */

int test_vault_run_hkdf_aead(ockam_vault_t* vault, ockam_memory_t* memory)
{
  int                           rc           = 0;
  char*                         test_name    = 0;
  uint16_t                      i            = 0;
  uint8_t*                      cmocka_data  = 0;
  struct CMUnitTest*            cmocka_tests = 0;
  test_vault_hkdf_aead_shared_data_t shared_data;

  ockam_error_t error =
    ockam_memory_alloc_zeroed(memory, (void**) &cmocka_data, (TEST_VAULT_HKDF_AEAD_TEST_CASES * sizeof(struct CMUnitTest)));
  if (ockam_error_has_error(&error)) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  shared_data.test_count     = 0;
  shared_data.test_count_max = TEST_VAULT_HKDF_AEAD_TEST_CASES;
  shared_data.vault          = vault;
  shared_data.memory         = memory;

  for (i = 0; i < TEST_VAULT_HKDF_AEAD_TEST_CASES; i++) {
    error = ockam_memory_alloc_zeroed(memory, (void**) &test_name, TEST_VAULT_HKDF_AEAD_NAME_SIZE);
    if (ockam_error_has_error(&error)) {
      rc = -1;
      goto exit_block;
    }

    snprintf(test_name, TEST_VAULT_HKDF_AEAD_NAME_SIZE, "HKDF Test Case %02d", i);

    cmocka_tests->name          = test_name;
    cmocka_tests->test_func     = test_vault_hkdf_aead;
    cmocka_tests->setup_func    = 0;
    cmocka_tests->teardown_func = test_vault_hkdf_aead_teardown;
    cmocka_tests->initial_state = &shared_data;

    cmocka_tests++;
  }

  if (ockam_error_has_error(&error)) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  rc = _cmocka_run_group_tests("HKDF", cmocka_tests, shared_data.test_count_max, 0, 0);

exit_block:
  return rc;
}
