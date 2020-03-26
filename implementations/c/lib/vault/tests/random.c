/**
 ********************************************************************************************************
 * @file    random.c
 * @brief   Ockam Vault common tests for random
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

#define TEST_VAULT_RAND_NUM_SIZE 32u /*!< Size of the random number to generate            */

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
 * @struct  TestVaultRandomSharedData
 * @brief   Shared test data for all unit tests
 *******************************************************************************
 */

typedef struct {
  const OckamVault *p_vault;
  const OckamMemory *p_memory;
  void *p_vault_ctx;
} TestVaultRandomSharedData;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void TestVaultRandom(void **state);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

uint8_t g_rand_num[TEST_VAULT_RAND_NUM_SIZE] = {0};

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
 *                                          TestVaultRandom()
 *
 * @brief   Ensure the specified ockam vault random function can generate a number
 *
 * @param   state   Shared variable between all test cases. Unused here.
 *
 ********************************************************************************************************
 */

void TestVaultRandom(void **state) {
  OckamError err = kOckamErrorNone;
  TestVaultRandomSharedData *p_test_data = 0;

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  p_test_data = (TestVaultRandomSharedData *)*state;

  err = p_test_data->p_vault->Random(p_test_data->p_vault_ctx, /* Generate a random number                           */
                                     (uint8_t *)&g_rand_num, TEST_VAULT_RAND_NUM_SIZE);
  assert_int_equal(err, kOckamErrorNone);
}

/**
 ********************************************************************************************************
 *                                          TestVaultRunRandom()
 *
 * @brief   Triggers the unit test for random number generation.
 *
 * @return  Zero on success. Non-zero on failure.
 *
 ********************************************************************************************************
 */

int TestVaultRunRandom(const OckamVault *p_vault, void *p_vault_ctx, const OckamMemory *p_memory) {
  TestVaultRandomSharedData shared_data;

  shared_data.p_vault = p_vault;
  shared_data.p_memory = p_memory;
  shared_data.p_vault_ctx = p_vault_ctx;

  const struct CMUnitTest tests[] = {
      cmocka_unit_test_prestate(TestVaultRandom, &shared_data),
  };

  return cmocka_run_group_tests_name("RANDOM", tests, 0, 0);
}
