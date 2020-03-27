/**
 ********************************************************************************************************
 * @file    hkdf.c
 * @brief   Common HKDF test functions for Ockam Vault
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
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

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define TEST_VAULT_HKDF_TEST_CASES 3u /*!< Total number of test cases to run                */
#define TEST_VAULT_HKDF_NAME_SIZE 32u /*!< Size of the buffer to allocate for the test name */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @struct  TestVaultHkdfData
 * @brief
 *******************************************************************************
 */
typedef struct {
  uint8_t *p_shared_secret;    /*!< Shared secret value to use for HKDF              */
  uint32_t shared_secret_size; /*!< Size of the shared secret value                  */
  uint8_t *p_salt;             /*!< Salt value for HKDF. Must fit into HW slot       */
  uint32_t salt_size;          /*!< Size of the salt value                           */
  uint8_t *p_info;             /*!< Optional info data for HKDF                      */
  uint32_t info_size;          /*!< Size of the info value                           */
  uint8_t *p_output;           /*!< Expected output from HKDF operation              */
  uint32_t output_size;        /*!< Size of the output to generate                   */
} TestVaultHkdfData;

/**
 *******************************************************************************
 * @struct  TestVaultHkdfSharedData
 * @brief   Shared test data for all unit tests
 *******************************************************************************
 */
typedef struct {
  uint16_t test_count;     /*!< Current unit test                                */
  uint16_t test_count_max; /*!< Total number of unit tests                       */
  const OckamVault *p_vault;
  const OckamMemory *p_memory;
  void *p_vault_ctx;
} TestVaultHkdfSharedData;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void TestVaultHkdf(void **state);
int TestVaultHkdfTeardown(void **state);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

uint8_t g_hkdf_test_1_shared_secret[] = {0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
                                         0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b};

uint8_t g_hkdf_test_1_salt[] = {0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c};

uint8_t g_hkdf_test_1_info[] = {0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9};

uint8_t g_hkdf_test_1_output[] = {0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
                                  0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
                                  0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65};

uint8_t g_hkdf_test_2_shared_secret[] = {0x37, 0xe0, 0xe7, 0xda, 0xac, 0xbd, 0x6b, 0xfb, 0xf6, 0x69, 0xa8,
                                         0x46, 0x19, 0x6f, 0xd4, 0x4d, 0x1c, 0x87, 0x45, 0xd3, 0x3f, 0x2b,
                                         0xe4, 0x2e, 0x31, 0xd4, 0x67, 0x41, 0x99, 0xad, 0x00, 0x5e};

uint8_t g_hkdf_test_2_salt[] = {0x4e, 0x6f, 0x69, 0x73, 0x65, 0x5f, 0x58, 0x58, 0x5f, 0x32, 0x35, 0x35, 0x31, 0x39,
                                0x5f, 0x41, 0x45, 0x53, 0x47, 0x43, 0x4d, 0x5f, 0x53, 0x48, 0x41, 0x32, 0x35, 0x36};

uint8_t g_hkdf_test_2_output[] = {0x67, 0x4A, 0xFE, 0x9E, 0x8A, 0x30, 0xE6, 0xDB, 0xF0, 0x73, 0xB3, 0x2C, 0xAD,
                                  0x4D, 0x71, 0x1D, 0x11, 0xED, 0xF3, 0x2A, 0x4B, 0x83, 0x47, 0x05, 0x83, 0xE6,
                                  0x89, 0x3B, 0xD4, 0x00, 0x41, 0xF4, 0xB8, 0x5A, 0xA7, 0xE2, 0xE0, 0x4A, 0x79,
                                  0x2D, 0x25, 0x3B, 0x95, 0x98, 0xED, 0x47, 0x60, 0x1A, 0x55, 0x46, 0x88, 0x13,
                                  0x09, 0x47, 0x8D, 0xF8, 0xD7, 0x0C, 0x54, 0x54, 0x32, 0x8A, 0x74, 0xC7};

uint8_t g_hkdf_test_3_salt[] = {0xde, 0xed, 0xe2, 0x5e, 0xee, 0x01, 0x58, 0xa0, 0xfd, 0xe9, 0x82,
                                0xe8, 0xbe, 0x1c, 0x79, 0x9d, 0x39, 0x5f, 0xd5, 0xba, 0xad, 0x40,
                                0x8c, 0x6b, 0xec, 0x2b, 0xa2, 0xe9, 0x0e, 0xb3, 0xc7, 0x18};

uint8_t g_hkdf_test_3_output[] = {0xb1, 0xc6, 0x74, 0xb6, 0x53, 0x5f, 0xb1, 0xd2, 0x08, 0x77, 0x2a, 0x97, 0x2c,
                                  0xac, 0x2c, 0xbf, 0x04, 0xd6, 0xaa, 0x08, 0x7c, 0xbb, 0xd3, 0xeb, 0x85, 0x58,
                                  0xa1, 0xa3, 0xab, 0xca, 0xa7, 0xfb, 0x10, 0x9c, 0x4b, 0x99, 0xea, 0x3a, 0x47,
                                  0x84, 0xff, 0x55, 0xaf, 0x5e, 0xed, 0x86, 0xc9, 0x9e, 0x85, 0x3f, 0x5a, 0x76,
                                  0xd8, 0x3c, 0xe4, 0x37, 0xa9, 0xe3, 0xe2, 0x7e, 0xde, 0x24, 0x2a, 0x6a};

TestVaultHkdfData g_hkdf_data[TEST_VAULT_HKDF_TEST_CASES] = {
    {&g_hkdf_test_1_shared_secret[0], 22, &g_hkdf_test_1_salt[0], 13, &g_hkdf_test_1_info[0], 10,
     &g_hkdf_test_1_output[0], 42},
    {&g_hkdf_test_2_shared_secret[0], 32, &g_hkdf_test_2_salt[0], 28, 0, 0, &g_hkdf_test_2_output[0], 64},
    {0, 0, &g_hkdf_test_3_salt[0], 32, 0, 0, &g_hkdf_test_3_output[0], 64},
};

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/**
 ********************************************************************************************************
 *                                          TestVaultHkdf()
 *
 * @brief   Common test functions for HKDF using Ockam Vault
 *
 ********************************************************************************************************
 */

void TestVaultHkdf(void **state) {
  OckamError err = kOckamErrorNone;
  TestVaultHkdfSharedData *p_test_data = 0;
  uint8_t *p_hkdf_key = 0;

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  p_test_data = (TestVaultHkdfSharedData *)*state;

  if (p_test_data->test_count >= p_test_data->test_count_max) {
    fail_msg("Test count %d has exceeded max test count of %d", p_test_data->test_count, p_test_data->test_count_max);
  }

  /* ----------------- */
  /* Memory Allocation */
  /* ----------------- */

  err = p_test_data->p_memory->Alloc((void **)&p_hkdf_key, g_hkdf_data[p_test_data->test_count].output_size);
  if (err != kOckamErrorNone) {
    fail_msg("Unable to allocate p_hkdf_key");
  }

  /* --------- */
  /* HKDF Test */
  /* --------- */

  err = p_test_data->p_vault->Hkdf(
      p_test_data->p_vault_ctx, g_hkdf_data[p_test_data->test_count].p_salt,
      g_hkdf_data[p_test_data->test_count].salt_size, g_hkdf_data[p_test_data->test_count].p_shared_secret,
      g_hkdf_data[p_test_data->test_count].shared_secret_size, g_hkdf_data[p_test_data->test_count].p_info,
      g_hkdf_data[p_test_data->test_count].info_size, p_hkdf_key, g_hkdf_data[p_test_data->test_count].output_size);
  assert_int_equal(err, kOckamErrorNone);

  assert_memory_equal(p_hkdf_key, g_hkdf_data[p_test_data->test_count].p_output,
                      g_hkdf_data[p_test_data->test_count].output_size);

  /* ----------- */
  /* Memory Free */
  /* ----------- */

  p_test_data->p_memory->Free(p_hkdf_key, g_hkdf_data[p_test_data->test_count].output_size);
}

/**
 ********************************************************************************************************
 *                                     TestVaultHkdfTeardown()
 *
 * @brief   Common unit test teardown function for HKDF using Ockam Vault
 *
 * @param   state   Contains a pointer to shared data for all HKDF test cases.
 *
 ********************************************************************************************************
 */

int TestVaultHkdfTeardown(void **state) {
  TestVaultHkdfSharedData *p_test_data = 0;

  /* ------------------- */
  /* Test Case Increment */
  /* ------------------- */

  p_test_data = (TestVaultHkdfSharedData *)*state;
  p_test_data->test_count++;

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

int TestVaultRunHkdf(const OckamVault *p_vault, void *p_vault_ctx, const OckamMemory *p_memory) {
  OckamError err = kOckamErrorNone;
  int rc = 0;
  char *p_test_name = 0;
  uint16_t i = 0;
  uint8_t *p_cmocka_data = 0;
  struct CMUnitTest *p_cmocka_tests = 0;
  TestVaultHkdfSharedData shared_data;

  err = p_memory->Alloc((void **)&p_cmocka_data, (TEST_VAULT_HKDF_TEST_CASES * sizeof(struct CMUnitTest)));
  if (err != kOckamErrorNone) {
    rc = -1;
    goto exit_block;
  }

  p_cmocka_tests = (struct CMUnitTest *)p_cmocka_data;

  shared_data.test_count = 0;
  shared_data.test_count_max = TEST_VAULT_HKDF_TEST_CASES;
  shared_data.p_vault = p_vault;
  shared_data.p_memory = p_memory;
  shared_data.p_vault_ctx = p_vault_ctx;

  for (i = 0; i < TEST_VAULT_HKDF_TEST_CASES; i++) {
    err = p_memory->Alloc((void **)&p_test_name, TEST_VAULT_HKDF_NAME_SIZE);
    if (err != kOckamErrorNone) {
      rc = -1;
      goto exit_block;
    }

    snprintf(p_test_name, TEST_VAULT_HKDF_NAME_SIZE, "HKDF Test Case %02d", i);

    p_cmocka_tests->name = p_test_name;
    p_cmocka_tests->test_func = TestVaultHkdf;
    p_cmocka_tests->setup_func = 0;
    p_cmocka_tests->teardown_func = TestVaultHkdfTeardown;
    p_cmocka_tests->initial_state = &shared_data;

    p_cmocka_tests++;
  }

  if (err != kOckamErrorNone) {
    rc = -1;
    goto exit_block;
  }

  p_cmocka_tests = (struct CMUnitTest *)p_cmocka_data;

  rc = _cmocka_run_group_tests("HKDF", p_cmocka_tests, shared_data.test_count_max, 0, 0);

exit_block:
  return rc;
}
