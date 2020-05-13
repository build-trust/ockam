/**
 * @file    hkdf.c
 * @brief   Common HKDF test functions for Ockam Vault
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

#define TEST_VAULT_HKDF_TEST_CASES          3u
#define TEST_VAULT_HKDF_NAME_SIZE           32u
#define TEST_VAULT_HKDF_DERIVED_OUTPUT_MAX  3u
#define TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE 32u

/**
 * @struct  test_vault_hkdf_data_t
 * @brief
 */
typedef struct {
  uint8_t* salt;
  uint32_t salt_size;
  uint8_t* ikm;
  uint32_t ikm_size;
  uint8_t* output;
  uint32_t output_count;
} test_vault_hkdf_data_t;

/**
 * @struct  test_vault_hkdf_shared_data_t
 * @brief   Shared test data for all unit tests
 */
typedef struct {
  uint16_t        test_count;
  uint16_t        test_count_max;
  ockam_vault_t*  vault;
  ockam_memory_t* memory;
} test_vault_hkdf_shared_data_t;

void test_vault_hkdf(void** state);
int  test_vault_hkdf_teardown(void** state);

/* clang-format off */

uint8_t g_hkdf_test_1_ikm[] =
{
  0x37, 0xe0, 0xe7, 0xda, 0xac, 0xbd, 0x6b, 0xfb,
  0xf6, 0x69, 0xa8, 0x46, 0x19, 0x6f, 0xd4, 0x4d,
  0x1c, 0x87, 0x45, 0xd3, 0x3f, 0x2b, 0xe4, 0x2e,
  0x31, 0xd4, 0x67, 0x41, 0x99, 0xad, 0x00, 0x5e
};

uint8_t g_hkdf_test_1_salt[] =
{
  0x4e, 0x6f, 0x69, 0x73, 0x65, 0x5f, 0x58, 0x58,
  0x5f, 0x32, 0x35, 0x35, 0x31, 0x39, 0x5f, 0x41,
  0x45, 0x53, 0x47, 0x43, 0x4d, 0x5f, 0x53, 0x48,
  0x41, 0x32, 0x35, 0x36
};

uint8_t g_hkdf_test_1_output[] =
{
  0x67, 0x4A, 0xFE, 0x9E, 0x8A, 0x30, 0xE6, 0xDB,
  0xF0, 0x73, 0xB3, 0x2C, 0xAD, 0x4D, 0x71, 0x1D,
  0x11, 0xED, 0xF3, 0x2A, 0x4B, 0x83, 0x47, 0x05,
  0x83, 0xE6, 0x89, 0x3B, 0xD4, 0x00, 0x41, 0xF4,
  0xB8, 0x5A, 0xA7, 0xE2, 0xE0, 0x4A, 0x79, 0x2D,
  0x25, 0x3B, 0x95, 0x98, 0xED, 0x47, 0x60, 0x1A,
  0x55, 0x46, 0x88, 0x13, 0x09, 0x47, 0x8D, 0xF8,
  0xD7, 0x0C, 0x54, 0x54, 0x32, 0x8A, 0x74, 0xC7
};

uint8_t g_hkdf_test_2_ikm[] =
{
  0x37, 0xe0, 0xe7, 0xda, 0xac, 0xbd, 0x6b, 0xfb,
  0xf6, 0x69, 0xa8, 0x46, 0x19, 0x6f, 0xd4, 0x4d,
  0x1c, 0x87, 0x45, 0xd3, 0x3f, 0x2b, 0xe4, 0x2e,
  0x31, 0xd4, 0x67, 0x41, 0x99, 0xad, 0x00, 0x5e
};

uint8_t g_hkdf_test_2_salt[] =
{
  0xde, 0xed, 0xe2, 0x5e, 0xee, 0x01, 0x58, 0xa0,
  0xfd, 0xe9, 0x82, 0xe8, 0xbe, 0x1c, 0x79, 0x9d,
  0x39, 0x5f, 0xd5, 0xba, 0xad, 0x40, 0x8c, 0x6b,
  0xec, 0x2b, 0xa2, 0xe9, 0x0e, 0xb3, 0xc7, 0x18
};

uint8_t g_hkdf_test_2_output[] =
{
  0x8a, 0xb6, 0x66, 0xfa, 0x91, 0xc8, 0x16, 0x96,
  0x7d, 0xbc, 0xb9, 0x78, 0xb4, 0x8c, 0x21, 0x65,
  0xc9, 0xb7, 0xf9, 0xcc, 0x76, 0xfe, 0xce, 0x03,
  0x2f, 0xde, 0x20, 0xd6, 0x0b, 0xcf, 0x36, 0x0d,
  0x82, 0x11, 0xf4, 0x4f, 0xf6, 0x8e, 0xac, 0x7a,
  0xf9, 0x36, 0x74, 0x39, 0x26, 0x99, 0x42, 0xde,
  0x98, 0x3a, 0x02, 0x8e, 0x41, 0x2d, 0xef, 0xd1,
  0x4b, 0x9e, 0x4c, 0x72, 0x0a, 0x6d, 0x3c, 0x5f,
  0x33, 0x70, 0x8f, 0x49, 0xe3, 0x11, 0x8a, 0x71,
  0x47, 0xc3, 0x20, 0x12, 0x7f, 0xf0, 0xd8, 0x75,
  0x9f, 0xa9, 0x57, 0xd3, 0x5d, 0x87, 0x6c, 0x48,
  0xb8, 0x99, 0x6c, 0x73, 0x89, 0x08, 0xa7, 0xe3
};

uint8_t g_hkdf_test_3_salt[] =
{
  0xDE, 0xED, 0xE2, 0x5E, 0xEE, 0x01, 0x58, 0xA0,
  0xFD, 0xE9, 0x82, 0xE8, 0xBE, 0x1C, 0x79, 0x9D,
  0x39, 0x5F, 0xD5, 0xBA, 0xAD, 0x40, 0x8C, 0x6B,
  0xEC, 0x2B, 0xA2, 0xE9, 0x0E, 0xB3, 0xC7, 0x18
};

uint8_t g_hkdf_test_3_output[] =
{
  0xB1, 0xC6, 0x74, 0xB6, 0x53, 0x5F, 0xB1, 0xD2,
  0x08, 0x77, 0x2A, 0x97, 0x2C, 0xAC, 0x2C, 0xBF,
  0x04, 0xD6, 0xAA, 0x08, 0x7C, 0xBB, 0xD3, 0xEB,
  0x85, 0x58, 0xA1, 0xA3, 0xAB, 0xCA, 0xA7, 0xFB,
  0x10, 0x9C, 0x4B, 0x99, 0xEA, 0x3A, 0x47, 0x84,
  0xFF, 0x55, 0xAF, 0x5E, 0xED, 0x86, 0xC9, 0x9E,
  0x85, 0x3F, 0x5A, 0x76, 0xD8, 0x3C, 0xE4, 0x37,
  0xA9, 0xE3, 0xE2, 0x7E, 0xDE, 0x24, 0x2A, 0x6A,
};

test_vault_hkdf_data_t g_hkdf_data[TEST_VAULT_HKDF_TEST_CASES] =
{
  {
    &g_hkdf_test_1_salt[0],
    28,
    &g_hkdf_test_1_ikm[0],
    32,
    &g_hkdf_test_1_output[0],
    2
  },
  {
    &g_hkdf_test_2_salt[0],
    32,
    &g_hkdf_test_2_ikm[0],
    32,
    &g_hkdf_test_2_output[0],
    3
  },
  {
    &g_hkdf_test_3_salt[0],
    32,
    0,
    0,
    &g_hkdf_test_3_output[0],
    2
  }
};

/* clang-format on */

/**
 * @brief   Common test functions for HKDF using Ockam Vault
 */

void test_vault_hkdf(void** state)
{
  ockam_error_t                  error     = OCKAM_ERROR_NONE;
  uint8_t                        i         = 0;
  uint8_t*                       hkdf_key  = 0;
  test_vault_hkdf_shared_data_t* test_data = 0;

  ockam_vault_secret_t            ikm_secret  = { 0 };
  ockam_vault_secret_t            salt_secret = { 0 };
  ockam_vault_secret_attributes_t attributes  = { 0 };

  ockam_vault_secret_t derived_outputs[TEST_VAULT_HKDF_DERIVED_OUTPUT_MAX] = { 0 };

  uint8_t generated_output[TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE] = { 0 };

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  test_data = (test_vault_hkdf_shared_data_t*) *state;

  if (test_data->test_count >= test_data->test_count_max) {
    fail_msg("Test count %d has exceeded max test count of %d", test_data->test_count, test_data->test_count_max);
  }

  /* --------- */
  /* HKDF Test */
  /* --------- */

  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  attributes.type        = OCKAM_VAULT_SECRET_TYPE_BUFFER;

  attributes.length = g_hkdf_data[test_data->test_count].salt_size;
  error             = ockam_vault_secret_import(test_data->vault,
                                    &salt_secret,
                                    &attributes,
                                    g_hkdf_data[test_data->test_count].salt,
                                    g_hkdf_data[test_data->test_count].salt_size);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  if (g_hkdf_data[test_data->test_count].ikm != 0) {
    attributes.length = g_hkdf_data[test_data->test_count].ikm_size;
    error             = ockam_vault_secret_import(test_data->vault,
                                      &ikm_secret,
                                      &attributes,
                                      g_hkdf_data[test_data->test_count].ikm,
                                      g_hkdf_data[test_data->test_count].ikm_size);
    assert_int_equal(error, OCKAM_ERROR_NONE);

    error = ockam_vault_hkdf_sha256(test_data->vault,
                                    &salt_secret,
                                    &ikm_secret,
                                    g_hkdf_data[test_data->test_count].output_count,
                                    &derived_outputs[0]);
    assert_int_equal(error, OCKAM_ERROR_NONE);
  } else {
    error = ockam_vault_hkdf_sha256(
      test_data->vault, &salt_secret, 0, g_hkdf_data[test_data->test_count].output_count, &derived_outputs[0]);
    assert_int_equal(error, OCKAM_ERROR_NONE);
  }

  for (i = 0; i < g_hkdf_data[test_data->test_count].output_count; i++) {
    size_t   length          = 0;
    uint8_t* expected_output = g_hkdf_data[test_data->test_count].output + (TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE * i);

    ockam_memory_set(test_data->memory, &generated_output[0], 0, TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE);

    error = ockam_vault_secret_export(
      test_data->vault, &derived_outputs[i], &generated_output[0], TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE, &length);
    assert_int_equal(error, OCKAM_ERROR_NONE);
    assert_int_equal(length, TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE);

    assert_memory_equal(&generated_output[0], expected_output, TEST_VAULT_HKDF_DERIVED_OUTPUT_SIZE);
  }
}

/**
 ********************************************************************************************************
 *                                     test_vault_hkdf_teardown()
 *
 * @brief   Common unit test teardown function for HKDF using Ockam Vault
 *
 * @param   state   Contains a pointer to shared data for all HKDF test cases.
 *
 ********************************************************************************************************
 */

int test_vault_hkdf_teardown(void** state)
{
  test_vault_hkdf_shared_data_t* test_data = 0;

  /* ------------------- */
  /* Test Case Increment */
  /* ------------------- */

  test_data = (test_vault_hkdf_shared_data_t*) *state;
  test_data->test_count++;

  return 0;
}

/**
 ********************************************************************************************************
 *                                          TestVaultRunHkdf()
 *
 * @brief   Triggers HKDF unit tests using Ockam Vault.
 *
 * @return  Zero on success. Non-zero on failure.
 *
 ********************************************************************************************************
 */

int test_vault_run_hkdf(ockam_vault_t* vault, ockam_memory_t* memory)
{
  ockam_error_t                 error        = OCKAM_ERROR_NONE;
  int                           rc           = 0;
  char*                         test_name    = 0;
  uint16_t                      i            = 0;
  uint8_t*                      cmocka_data  = 0;
  struct CMUnitTest*            cmocka_tests = 0;
  test_vault_hkdf_shared_data_t shared_data;

  error =
    ockam_memory_alloc_zeroed(memory, (void**) &cmocka_data, (TEST_VAULT_HKDF_TEST_CASES * sizeof(struct CMUnitTest)));
  if (error != OCKAM_ERROR_NONE) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  shared_data.test_count     = 0;
  shared_data.test_count_max = TEST_VAULT_HKDF_TEST_CASES;
  shared_data.vault          = vault;
  shared_data.memory         = memory;

  for (i = 0; i < TEST_VAULT_HKDF_TEST_CASES; i++) {
    error = ockam_memory_alloc_zeroed(memory, (void**) &test_name, TEST_VAULT_HKDF_NAME_SIZE);
    if (error != OCKAM_ERROR_NONE) {
      rc = -1;
      goto exit_block;
    }

    snprintf(test_name, TEST_VAULT_HKDF_NAME_SIZE, "HKDF Test Case %02d", i);

    cmocka_tests->name          = test_name;
    cmocka_tests->test_func     = test_vault_hkdf;
    cmocka_tests->setup_func    = 0;
    cmocka_tests->teardown_func = test_vault_hkdf_teardown;
    cmocka_tests->initial_state = &shared_data;

    cmocka_tests++;
  }

  if (error != OCKAM_ERROR_NONE) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  rc = _cmocka_run_group_tests("HKDF", cmocka_tests, shared_data.test_count_max, 0, 0);

exit_block:
  return rc;
}
