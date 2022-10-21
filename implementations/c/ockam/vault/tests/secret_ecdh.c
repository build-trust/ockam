/**
 * @file    secret_ecdh.c
 * @brief   Ockam Vault common tests for key generation and ECDH
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

#define TEST_VAULT_KEY_NAME_SIZE             32u
#define TEST_VAULT_KEY_P256_TEST_CASES       1u
#define TEST_VAULT_KEY_CURVE25519_TEST_CASES 2u
#define TEST_VAULT_KEY_PRIV_SIZE             32u

/**
 * @struct  test_vault_keys_p256_t
 * @brief   Initiator and responder test keys on P256
 */
typedef struct {
  uint8_t initiator_priv[TEST_VAULT_KEY_PRIV_SIZE];
  uint8_t initiator_pub[OCKAM_VAULT_P256_PUBLICKEY_LENGTH];
  uint8_t responder_priv[TEST_VAULT_KEY_PRIV_SIZE];
  uint8_t responder_pub[OCKAM_VAULT_P256_PUBLICKEY_LENGTH];
  uint8_t shared_secret[OCKAM_VAULT_SHARED_SECRET_LENGTH];
} test_vault_keys_p256_t;

/**
 * @struct  test_vault_keys_curve25519_t
 * @brief   Initiator and responder test keys on Curve25519
 */
typedef struct {
  uint8_t initiator_priv[TEST_VAULT_KEY_PRIV_SIZE];
  uint8_t initiator_pub[OCKAM_VAULT_CURVE25519_PUBLICKEY_LENGTH];
  uint8_t responder_priv[TEST_VAULT_KEY_PRIV_SIZE];
  uint8_t responder_pub[OCKAM_VAULT_CURVE25519_PUBLICKEY_LENGTH];
  uint8_t shared_secret[OCKAM_VAULT_SHARED_SECRET_LENGTH];
} test_vault_keys_curve25519_t;

/**
 * @struct  test_vault_key_shared_data_t
 * @brief   Global test data for each test run
 */
typedef struct {
  ockam_vault_t*            vault;
  ockam_memory_t*           memory;
  ockam_vault_secret_type_t type;
  uint16_t                  test_count;
  uint16_t                  test_count_max;
  uint8_t                   load_keys;
  uint8_t                   key_size;
} test_vault_key_shared_data_t;

void test_vault_secret_ecdh(void** state);
int  test_vault_secret_ecdh_teardown(void** state);

/* clang-format off */

test_vault_keys_p256_t g_test_vault_keys_p256[TEST_VAULT_KEY_P256_TEST_CASES] =
{
  {
    {
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Case 0: Initiator Private Key */
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    },
    {
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Case 0: Initiator Public Key */
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    },
    {
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Case 0: Responder Private Key */
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    },
    {
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Case 0: Responder Public Key */
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    },
    {
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Case 0: Expected Shared Secret */
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    },
  },
};


test_vault_keys_curve25519_t g_test_vault_keys_curve25519[TEST_VAULT_KEY_CURVE25519_TEST_CASES] =
{
  {
    {
      0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, /* Case 0: Initiator Private Key */
      0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
      0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
      0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f
    },
    {
      0x8f, 0x40, 0xc5, 0xad, 0xb6, 0x8f, 0x25, 0x62, /* Case 0: Initiator Public Key */
      0x4a, 0xe5, 0xb2, 0x14, 0xea, 0x76, 0x7a, 0x6e,
      0xc9, 0x4d, 0x82, 0x9d, 0x3d, 0x7b, 0x5e, 0x1a,
      0xd1, 0xba, 0x6f, 0x3e, 0x21, 0x38, 0x28, 0x5f
    },
    {
      0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, /* Case 0: Responder Private Key */
      0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
      0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
      0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
    },
    {
      0x07, 0xa3, 0x7c, 0xbc, 0x14, 0x20, 0x93, 0xc8, /* Case 0: Responder Public Key */
      0xb7, 0x55, 0xdc, 0x1b, 0x10, 0xe8, 0x6c, 0xb4,
      0x26, 0x37, 0x4a, 0xd1, 0x6a, 0xa8, 0x53, 0xed,
      0x0b, 0xdf, 0xc0, 0xb2, 0xb8, 0x6d, 0x1c, 0x7c
    },
    {
      0x42, 0x74, 0xA3, 0x2E, 0x95, 0x3A, 0xCB, 0x83, /* Case 0: Expected Shared Secret Value */
      0x14, 0xD0, 0xF0, 0x9B, 0xCB, 0xCB, 0x51, 0x93,
      0xC5, 0xEF, 0x79, 0x9D, 0xDC, 0xD0, 0x03, 0x6F,
      0x8C, 0x46, 0x82, 0xE5, 0x80, 0x1D, 0xAC, 0x73
    }
  },
  {
    {
      0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, /* Case 1: Initiator Private Key */
      0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
      0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
      0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
    },
    {
      0x35, 0x80, 0x72, 0xd6, 0x36, 0x58, 0x80, 0xd1, /* Case 1: Initiator Public Key */
      0xae, 0xea, 0x32, 0x9a, 0xdf, 0x91, 0x21, 0x38,
      0x38, 0x51, 0xed, 0x21, 0xa2, 0x8e, 0x3b, 0x75,
      0xe9, 0x65, 0xd0, 0xd2, 0xcd, 0x16, 0x62, 0x54
    },
    {
      0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, /* Case 1: Responder Private Key */
      0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
      0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
      0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60
    },
    {
      0x64, 0xb1, 0x01, 0xb1, 0xd0, 0xbe, 0x5a, 0x87, /* Case 1: Responder Public Key */
      0x04, 0xbd, 0x07, 0x8f, 0x98, 0x95, 0x00, 0x1f,
      0xc0, 0x3e, 0x8e, 0x9f, 0x95, 0x22, 0xf1, 0x88,
      0xdd, 0x12, 0x8d, 0x98, 0x46, 0xd4, 0x84, 0x66
    },
    {
      0x37, 0xE0, 0xE7, 0xDA, 0xAC, 0xBD, 0x6B, 0xFB, /* Case 1: Expected Shared Secret Value */
      0xF6, 0x69, 0xA8, 0x46, 0x19, 0x6F, 0xD4, 0x4D,
      0x1C, 0x87, 0x45, 0xD3, 0x3F, 0x2B, 0xE4, 0x2E,
      0x31, 0xD4, 0x67, 0x41, 0x99, 0xAD, 0x00, 0x5E
    }
  },
};

/* clang-format on */

const char g_test_vault_p256_name[]       = "P-256: ";
const char g_test_vault_curve25519_name[] = "Curve25519: ";

/**
 * @brief   Main unit test for Key/ECDH. Tests private key write/generate, public key retrieval, and
 *          ECDH. In cases where private keys were written to the device, public key data and shared
 *          secrets are validated against known values.
 *
 * @param   state   Contains the shared test data used in all Key/ECDH unit tests.
 */

void test_vault_secret_ecdh(void** state)
{
  test_vault_key_shared_data_t* test_data = 0;
  size_t                        length    = 0;

  ockam_vault_secret_t            initiator_secret = { 0 };
  ockam_vault_secret_t            responder_secret = { 0 };
  ockam_vault_secret_t            shared_secret_0  = { 0 };
  ockam_vault_secret_t            shared_secret_1  = { 0 };
  ockam_vault_secret_attributes_t attributes       = { 0 };

  uint8_t* initiator_priv = 0;
  uint8_t* initiator_pub  = 0;
  uint8_t* responder_priv = 0;
  uint8_t* responder_pub  = 0;
  uint8_t* shared_secret  = 0;

  uint8_t* generated_initiator_pub = 0;
  uint8_t* generated_responder_pub = 0;

  uint8_t generated_shared_secret_0[OCKAM_VAULT_SHARED_SECRET_LENGTH] = { 0 };
  uint8_t generated_shared_secret_1[OCKAM_VAULT_SHARED_SECRET_LENGTH] = { 0 };

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  test_data = (test_vault_key_shared_data_t*) *state;

  if (test_data->test_count >= test_data->test_count_max) {
    fail_msg("Test count %d has exceeded max tests of %d", test_data->test_count, test_data->test_count_max);
  }

  /* ----------------- */
  /* Memory Allocation */
  /* ----------------- */

  ockam_error_t error = ockam_memory_alloc_zeroed(test_data->memory, (void**) &generated_initiator_pub, test_data->key_size);
  if (ockam_error_has_error(&error)) { fail_msg("Unable to alloc generated_initiator_pub"); }

  error = ockam_memory_alloc_zeroed(test_data->memory, (void**) &generated_responder_pub, test_data->key_size);
  if (ockam_error_has_error(&error)) { fail_msg("Unable to alloc generated_responder_pub"); }

  /* ------------------ */
  /* Key Write/Generate */
  /* ------------------ */

  attributes.length      = 0;
  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;

  if (test_data->type == OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) {
    initiator_priv = &g_test_vault_keys_p256[test_data->test_count].initiator_priv[0];
    initiator_pub  = &g_test_vault_keys_p256[test_data->test_count].initiator_pub[0];
    responder_priv = &g_test_vault_keys_p256[test_data->test_count].responder_priv[0];
    responder_pub  = &g_test_vault_keys_p256[test_data->test_count].responder_pub[0];
    shared_secret  = &g_test_vault_keys_p256[test_data->test_count].shared_secret[0];

    attributes.type = OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY;
  } else if (test_data->type == OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY) {
    initiator_priv = &g_test_vault_keys_curve25519[test_data->test_count].initiator_priv[0];
    initiator_pub  = &g_test_vault_keys_curve25519[test_data->test_count].initiator_pub[0];
    responder_priv = &g_test_vault_keys_curve25519[test_data->test_count].responder_priv[0];
    responder_pub  = &g_test_vault_keys_curve25519[test_data->test_count].responder_pub[0];
    shared_secret  = &g_test_vault_keys_curve25519[test_data->test_count].shared_secret[0];

    attributes.type = OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY;
  }

  if (test_data->load_keys) {
    error = ockam_vault_secret_import(
      test_data->vault, &initiator_secret, &attributes, initiator_priv, TEST_VAULT_KEY_PRIV_SIZE);
    assert_true(ockam_error_is_none(&error));

    error = ockam_vault_secret_import(
      test_data->vault, &responder_secret, &attributes, responder_priv, TEST_VAULT_KEY_PRIV_SIZE);
    assert_true(ockam_error_is_none(&error));

  } else {
    error = ockam_vault_secret_generate(test_data->vault, &initiator_secret, &attributes);
    assert_true(ockam_error_is_none(&error));

    error = ockam_vault_secret_generate(test_data->vault, &responder_secret, &attributes);
    assert_true(ockam_error_is_none(&error));
  }

  /* ------------- */
  /* Key Retrieval */
  /* ------------- */

  error = ockam_vault_secret_publickey_get(
    test_data->vault, &initiator_secret, generated_initiator_pub, test_data->key_size, &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, test_data->key_size);

  error = ockam_vault_secret_publickey_get(
    test_data->vault, &responder_secret, generated_responder_pub, test_data->key_size, &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, test_data->key_size);

  if (test_data->load_keys) {
    assert_memory_equal(generated_initiator_pub, initiator_pub, test_data->key_size);
    assert_memory_equal(generated_responder_pub, responder_pub, test_data->key_size);
  }

  /* ----------------- */
  /* ECDH Calculations */
  /* ----------------- */

  error = ockam_vault_ecdh(
    test_data->vault, &initiator_secret, generated_responder_pub, test_data->key_size, &shared_secret_0);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_export(
    test_data->vault, &shared_secret_0, &generated_shared_secret_0[0], OCKAM_VAULT_SHARED_SECRET_LENGTH, &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, OCKAM_VAULT_SHARED_SECRET_LENGTH);

  error = ockam_vault_ecdh(
    test_data->vault, &responder_secret, generated_initiator_pub, test_data->key_size, &shared_secret_1);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_export(
    test_data->vault, &shared_secret_1, &generated_shared_secret_1[0], OCKAM_VAULT_SHARED_SECRET_LENGTH, &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, OCKAM_VAULT_SHARED_SECRET_LENGTH);

  assert_memory_equal(&generated_shared_secret_0[0], &generated_shared_secret_1[0], OCKAM_VAULT_SHARED_SECRET_LENGTH);

  if (test_data->load_keys) {
    assert_memory_equal(&generated_shared_secret_0[0], shared_secret, OCKAM_VAULT_SHARED_SECRET_LENGTH);
  }

  /* ----------- */
  /* Memory free */
  /* ----------- */

  error = ockam_vault_secret_destroy(test_data->vault, &initiator_secret);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_destroy(test_data->vault, &responder_secret);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_destroy(test_data->vault, &shared_secret_0);
  assert_true(ockam_error_is_none(&error));

  error = ockam_vault_secret_destroy(test_data->vault, &shared_secret_1);
  assert_true(ockam_error_is_none(&error));

  ockam_memory_free(test_data->memory, generated_initiator_pub, test_data->key_size);
  ockam_memory_free(test_data->memory, generated_responder_pub, test_data->key_size);
}

/**
 * @brief   Common unit test teardown function for Key/Ecdh using Ockam Vault
 *
 * @param   state   Contains a pointer to shared data for all Key/Ecdh test cases.
 */

int test_vault_secret_ecdh_teardown(void** state)
{
  test_vault_key_shared_data_t* test_data = 0;

  /* ------------------- */
  /* Test Case Increment */
  /* ------------------- */

  test_data = (test_vault_key_shared_data_t*) *state;
  test_data->test_count++;

  return 0;
}

/**
 * @brief   Triggers the unit tests for Key/ECDH depending on the type of elliptic curve specified
 *
 * @param   ec          The elliptic curve to run the tests on.
 *
 * @param   load_keys   If >0, the selected platform supports writing private keys to the device. In
 *                      this case the unit test takes advantage of writing a private key to the
 *                      specified Vault and validates the resulting public key and shared secrets. If 0,
 *                      private keys will be randomly generated and the only check performed is that the
 *                      resulting shared secrets match.
 *
 * @return  0 on success, non-zero on failure.
 */
int test_vault_run_secret_ecdh(ockam_vault_t*            vault,
                               ockam_memory_t*           memory,
                               ockam_vault_secret_type_t type,
                               uint8_t                   load_keys)
{
  int           rc    = 0;

  uint8_t                      i            = 0;
  char*                        name         = 0;
  char*                        test_name    = 0;
  uint8_t*                     cmocka_data  = 0;
  struct CMUnitTest*           cmocka_tests = 0;
  test_vault_key_shared_data_t test_data    = { 0 };

  test_data.vault      = vault;
  test_data.memory     = memory;
  test_data.type       = type;
  test_data.load_keys  = load_keys;
  test_data.test_count = 0;

  if (type == OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) {
    test_data.test_count_max = TEST_VAULT_KEY_P256_TEST_CASES;
    test_data.key_size       = OCKAM_VAULT_P256_PUBLICKEY_LENGTH;
    name                     = (char*) &g_test_vault_p256_name[0];
  } else if (type == OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY) {
    test_data.test_count_max = TEST_VAULT_KEY_CURVE25519_TEST_CASES;
    test_data.key_size       = OCKAM_VAULT_CURVE25519_PUBLICKEY_LENGTH;
    name                     = (char*) &g_test_vault_curve25519_name[0];
  } else {
    rc = -1;
    goto exit_block;
  }

  ockam_error_t error =
    ockam_memory_alloc_zeroed(memory, (void**) &cmocka_data, test_data.test_count_max * sizeof(struct CMUnitTest));
  if (ockam_error_has_error(&error)) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  for (i = 0; i < test_data.test_count_max; i++) {
    error = ockam_memory_alloc_zeroed(memory, (void**) &test_name, TEST_VAULT_KEY_NAME_SIZE);
    if (ockam_error_has_error(&error)) { break; }

    snprintf(test_name, TEST_VAULT_KEY_NAME_SIZE, "%s Test Case %02d", name, i);

    cmocka_tests->name          = test_name;
    cmocka_tests->test_func     = test_vault_secret_ecdh;
    cmocka_tests->setup_func    = 0;
    cmocka_tests->teardown_func = test_vault_secret_ecdh_teardown;
    cmocka_tests->initial_state = &test_data;

    cmocka_tests++;
  }

  if (ockam_error_has_error(&error)) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  rc = _cmocka_run_group_tests("KEY_ECDH", cmocka_tests, test_data.test_count_max, 0, 0);

exit_block:
  return rc;
}
