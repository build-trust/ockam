/**
 * @file    random.c
 * @brief   Ockam Vault common tests for random
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

#define TEST_VAULT_RAND_NUM_SIZE 32u

/**
 * @struct  TestVaultRandomSharedData
 * @brief   Shared test data for all unit tests
 */
typedef struct {
  ockam_vault_t* vault;
} test_vault_random_shared_data_t;

void test_vault_random(void** state);

uint8_t g_rand_num[TEST_VAULT_RAND_NUM_SIZE] = { 0 };

/**
 * @brief   Ensure the specified ockam vault random function can generate a number
 * @param   state   Shared variable between all test cases. Unused here.
 */
void test_vault_random(void** state)
{
  ockam_error_t                    error     = OCKAM_ERROR_NONE;
  test_vault_random_shared_data_t* test_data = 0;

  test_data = (test_vault_random_shared_data_t*) *state;

  error = ockam_vault_random_bytes_generate(test_data->vault, &g_rand_num[0], TEST_VAULT_RAND_NUM_SIZE);
  assert_int_equal(error, OCKAM_ERROR_NONE);
}

/**
 * @brief   Triggers the unit test for random number generation.
 * @return  Zero on success. Non-zero on failure.
 */
int test_vault_run_random(ockam_vault_t* vault, ockam_memory_t* memory)
{
  test_vault_random_shared_data_t shared_data;

  shared_data.vault = vault;

  const struct CMUnitTest tests[] = {
    cmocka_unit_test_prestate(test_vault_random, &shared_data),
  };

  return cmocka_run_group_tests_name("RANDOM", tests, 0, 0);
}
