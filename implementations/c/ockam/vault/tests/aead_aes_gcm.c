/**
 * @file    aead_aes_gcm.c
 * @brief   Common AES GCM test cases for Ockam Vault
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

#define TEST_VAULT_AEAD_AES_GCM_TEST_CASES 4u
#define TEST_VAULT_AEAD_AES_GCM_NAME_SIZE  32u
#define TEST_VAULT_AEAD_AES_GCM_TAG_SIZE   16u

#define TEST_VAULT_AEAD_AES_GCM_128_KEY_SIZE 16u
#define TEST_VAULT_AEAD_AES_GCM_256_KEY_SIZE 32u

/**
 * @struct  test_vault_aead_aes_gcm_data_t
 * @brief   Common AES GCM test data
 */
typedef struct {
  uint8_t* key;
  uint8_t  key_size;
  uint8_t* aad;
  uint32_t aad_size;
  uint64_t nonce;
  uint8_t* plaintext;
  uint8_t* ciphertext_and_tag;
  uint32_t text_size;
} test_vault_aead_aes_gcm_data_t;

/**
 * @struct  test_vault_aead_aes_gcm_shared_data_t
 * @brief   Shared test data for all unit tests
 */
typedef struct {
  uint16_t                      test_count;
  uint16_t                      test_count_max;
  ockam_vault_t*                vault;
  ockam_memory_t*               memory;
  TEST_VAULT_AEAD_AES_GCM_KEY_e test_key_type;
} test_vault_aead_aes_gcm_shared_data_t;

void test_vault_aead_aes_gcm(void** state);
int  test_vault_aead_aes_gcm_teardown(void** state);

/* clang-format off */

uint8_t g_aead_aes_gcm_test_key_128[] =
{
  0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c,
  0x6d, 0x6a, 0x8f, 0x94, 0x67, 0x30, 0x83, 0x08
};

uint8_t g_aead_aes_gcm_test_key_256[] =
{
  0xc5, 0x02, 0x74, 0xde, 0x93, 0xe9, 0x96, 0xb6,
  0x61, 0xf1, 0xa6, 0xf1, 0xeb, 0x7d, 0xaa, 0x9d,
  0xda, 0xbf, 0x1d, 0xe2, 0x0a, 0x83, 0xd3, 0xbf,
  0xa6, 0xdb, 0xe3, 0xb9, 0x22, 0x02, 0x2a, 0x48
};

uint8_t g_aead_aes_gcm_test_aad[] =
{
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xab, 0xad, 0xda, 0xd2
};

uint8_t g_aead_aes_gcm_test_plaintext_long[] =
{
  0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24,
  0x4b, 0x72, 0x21, 0xb7, 0x84, 0xd0, 0xd4, 0x9c,
  0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02, 0xa4, 0xe0,
  0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e,
  0x21, 0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c,
  0x7d, 0x8f, 0x6a, 0x5a, 0xac, 0x84, 0xaa, 0x05,
  0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac, 0x97,
  0x3d, 0x58, 0xe0, 0x91
};

uint8_t g_aead_aes_gcm_test_plaintext_short[] =
{
  0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
  0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F
};

uint8_t g_aead_aes_gcm_test_ciphertext_and_tag_128_long[] =
{
  0xBA, 0x03, 0xED, 0xE8, 0x35, 0xAE, 0x90, 0x54,
  0x6D, 0xB8, 0xDD, 0x94, 0x0C, 0x82, 0x23, 0xD4,
  0xDA, 0x27, 0xC2, 0x11, 0x33, 0x22, 0x4F, 0x33,
  0xC0, 0xC7, 0x0F, 0x59, 0xD1, 0x34, 0xB2, 0x81,
  0xC9, 0xB3, 0xF3, 0x27, 0x24, 0x86, 0x66, 0xEC,
  0xFA, 0x27, 0x78, 0x2D, 0x85, 0xC8, 0xCF, 0x4B,
  0x11, 0xCF, 0xE7, 0x11, 0x3C, 0xC4, 0x6D, 0x82,
  0x7F, 0x36, 0x7D, 0xAB, 0x3F, 0xB5, 0xA7, 0x9E,
  0xB4, 0xDB, 0x85, 0x89, 0x12, 0x83, 0x66, 0x54,
  0x86, 0x3E, 0xA1, 0x69
};

uint8_t g_aead_aes_gcm_test_ciphertext_and_tag_128_short[] =
{
  0xF8, 0x81, 0xF1, 0x29, 0x10, 0xDC, 0xE2, 0x77,
  0x2E, 0xC3, 0xF6, 0x28, 0x84, 0x5F, 0xF9, 0x47,
  0x50, 0x78, 0xDB, 0x0F, 0x96, 0x70, 0x05, 0x5A,
  0x1A, 0xD5, 0xC8, 0xBF, 0x65, 0x86, 0x3B, 0x70
};

uint8_t g_aead_aes_gcm_test_ciphertext_and_tag_256_long[] =
{
  0x90, 0x94, 0xBB, 0x7D, 0x29, 0xDE, 0x51, 0xC8,
  0xF9, 0xFB, 0x99, 0xE4, 0xA8, 0x29, 0x82, 0xD6,
  0xE9, 0x22, 0x17, 0x56, 0x65, 0x94, 0x83, 0x4D,
  0x1D, 0x47, 0x55, 0xDA, 0x3F, 0x81, 0xE8, 0x0C,
  0xAB, 0x80, 0xB0, 0x51, 0x2C, 0x1B, 0x55, 0xAB,
  0x06, 0x00, 0xB7, 0x5B, 0xAE, 0x20, 0xBD, 0x0A,
  0xBC, 0xAE, 0xC8, 0x09, 0x91, 0x07, 0xEA, 0x23,
  0x40, 0x56, 0xE9, 0x24, 0xCF, 0x71, 0x04, 0x93,
  0x0C, 0xB4, 0x7F, 0x19, 0xA6, 0x2C, 0x4B, 0xE7,
  0x94, 0x33, 0x81, 0x9D,
};

uint8_t g_aead_aes_gcm_test_ciphertext_and_tag_256_short[] =
{
  0xd2, 0x16, 0xa7, 0xbc, 0x0c, 0xac, 0x23, 0xeb,
  0xba, 0x80, 0xb2, 0x58, 0x20, 0xf4, 0x58, 0x45,
  0x30, 0xb2, 0x7b, 0x53, 0x3c, 0x52, 0x84, 0x81,
  0xb3, 0xf6, 0x27, 0x27, 0x4d, 0xfc, 0xa1, 0xc3
};


test_vault_aead_aes_gcm_data_t g_aead_aes_gcm_data[TEST_VAULT_AEAD_AES_GCM_TEST_CASES] =
{
  {
    &g_aead_aes_gcm_test_key_128[0],
    TEST_VAULT_AEAD_AES_GCM_128_KEY_SIZE,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext_long[0],
    &g_aead_aes_gcm_test_ciphertext_and_tag_128_long[0],
    60,
  },
  {
    &g_aead_aes_gcm_test_key_128[0],
    TEST_VAULT_AEAD_AES_GCM_128_KEY_SIZE,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext_short[0],
    &g_aead_aes_gcm_test_ciphertext_and_tag_128_short[0],
    16,
  },
  {
    &g_aead_aes_gcm_test_key_256[0],
    TEST_VAULT_AEAD_AES_GCM_256_KEY_SIZE,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext_long[0],
    &g_aead_aes_gcm_test_ciphertext_and_tag_256_long[0],
    60,
  },
  {
    &g_aead_aes_gcm_test_key_256[0],
    TEST_VAULT_AEAD_AES_GCM_256_KEY_SIZE,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext_short[0],
    &g_aead_aes_gcm_test_ciphertext_and_tag_256_short[0],
    16,
  },
};

/* clang-format on */

/**
 * @brief   Run through encryption and decryption test cases using Ockam Vault
 * @param   state   Contains a shared data pointer for common test data.
 */
void test_vault_aead_aes_gcm(void** state)
{
  test_vault_aead_aes_gcm_shared_data_t* test_data                      = 0;
  size_t                                 length                         = 0;
  size_t                                 aead_aes_gcm_encrypt_hash_size = 0;
  size_t                                 aead_aes_gcm_decrypt_data_size = 0;
  uint8_t*                               aead_aes_gcm_encrypt_hash      = 0;
  uint8_t*                               aead_aes_gcm_decrypt_data      = 0;
  ockam_vault_secret_t                   key_secret                     = { 0 };
  ockam_vault_secret_attributes_t        attributes                     = { 0 };

  /* -------------------------- */
  /* Test Data and Verification */
  /* -------------------------- */

  test_data = (test_vault_aead_aes_gcm_shared_data_t*) *state;

  if (test_data->test_count >= test_data->test_count_max) {
    fail_msg("Test count %d has exceeded max test count of %d", test_data->test_count, test_data->test_count_max);
  }

  if ((test_data->test_key_type == TEST_VAULT_AEAD_AES_GCM_KEY_128_ONLY) &&
      (g_aead_aes_gcm_data[test_data->test_count].key_size == TEST_VAULT_AEAD_AES_GCM_256_KEY_SIZE)) {
    goto exit;
  }

  if ((test_data->test_key_type == TEST_VAULT_AEAD_AES_GCM_KEY_256_ONLY) &&
      (g_aead_aes_gcm_data[test_data->test_count].key_size == TEST_VAULT_AEAD_AES_GCM_128_KEY_SIZE)) {
    goto exit;
  }

  /* ----------------- */
  /* Memory Allocation */
  /* ----------------- */

  if (g_aead_aes_gcm_data[test_data->test_count].text_size > 0) {
    aead_aes_gcm_encrypt_hash_size =
      g_aead_aes_gcm_data[test_data->test_count].text_size + TEST_VAULT_AEAD_AES_GCM_TAG_SIZE;

    ockam_error_t error =
      ockam_memory_alloc_zeroed(test_data->memory, (void**) &aead_aes_gcm_encrypt_hash, aead_aes_gcm_encrypt_hash_size);
    if (ockam_error_has_error(&error)) { fail_msg("Unable to alloc aead_aes_gcm_encrypt_hash"); }

    aead_aes_gcm_decrypt_data_size = g_aead_aes_gcm_data[test_data->test_count].text_size;

    error =
      ockam_memory_alloc_zeroed(test_data->memory, (void**) &aead_aes_gcm_decrypt_data, aead_aes_gcm_decrypt_data_size);
    if (ockam_error_has_error(&error)) { fail_msg("Unable to alloc aead_aes_gcm_decrypt_data"); }
  }

  /* ------- */
  /* AES Key */
  /* ------- */

  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  attributes.type        = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  attributes.length      = g_aead_aes_gcm_data[test_data->test_count].key_size;

  ockam_error_t error = ockam_vault_secret_import(test_data->vault,
                                    &key_secret,
                                    &attributes,
                                    g_aead_aes_gcm_data[test_data->test_count].key,
                                    g_aead_aes_gcm_data[test_data->test_count].key_size);
  assert_true(ockam_error_is_none(&error));

  if (attributes.length == OCKAM_VAULT_AES128_KEY_LENGTH) {
    error = ockam_vault_secret_type_set(test_data->vault, &key_secret, OCKAM_VAULT_SECRET_TYPE_AES128_KEY);
    assert_true(ockam_error_is_none(&error));
  } else if (attributes.length == OCKAM_VAULT_AES256_KEY_LENGTH) {
    error = ockam_vault_secret_type_set(test_data->vault, &key_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
    assert_true(ockam_error_is_none(&error));
  } else {
    fail_msg("Invalid AES key specified");
  }

  /* --------------- */
  /* AES GCM Encrypt */
  /* --------------- */

  error = ockam_vault_aead_aes_gcm_encrypt(test_data->vault,
                                           &key_secret,
                                           g_aead_aes_gcm_data[test_data->test_count].nonce,
                                           g_aead_aes_gcm_data[test_data->test_count].aad,
                                           g_aead_aes_gcm_data[test_data->test_count].aad_size,
                                           g_aead_aes_gcm_data[test_data->test_count].plaintext,
                                           g_aead_aes_gcm_data[test_data->test_count].text_size,
                                           aead_aes_gcm_encrypt_hash,
                                           aead_aes_gcm_encrypt_hash_size,
                                           &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, aead_aes_gcm_encrypt_hash_size);

  assert_memory_equal(aead_aes_gcm_encrypt_hash,
                      g_aead_aes_gcm_data[test_data->test_count].ciphertext_and_tag,
                      aead_aes_gcm_encrypt_hash_size);

  /* --------------- */
  /* AES GCM Decrypt */
  /* --------------- */

  error = ockam_vault_aead_aes_gcm_decrypt(test_data->vault,
                                           &key_secret,
                                           g_aead_aes_gcm_data[test_data->test_count].nonce,
                                           g_aead_aes_gcm_data[test_data->test_count].aad,
                                           g_aead_aes_gcm_data[test_data->test_count].aad_size,
                                           g_aead_aes_gcm_data[test_data->test_count].ciphertext_and_tag,
                                           aead_aes_gcm_encrypt_hash_size,
                                           aead_aes_gcm_decrypt_data,
                                           aead_aes_gcm_decrypt_data_size,
                                           &length);
  assert_true(ockam_error_is_none(&error));
  assert_int_equal(length, aead_aes_gcm_decrypt_data_size);

  assert_memory_equal(
    aead_aes_gcm_decrypt_data, g_aead_aes_gcm_data[test_data->test_count].plaintext, aead_aes_gcm_decrypt_data_size);

  /* ----------- */
  /* Memory Free */
  /* ----------- */

  // TODO this will not be freed on an error
  ockam_memory_free(test_data->memory, aead_aes_gcm_encrypt_hash, aead_aes_gcm_encrypt_hash_size);
  ockam_memory_free(test_data->memory, aead_aes_gcm_decrypt_data, g_aead_aes_gcm_data[test_data->test_count].text_size);

exit:
  return;
}

/**
 * @brief   Common unit test teardown function for AES GCM using Ockam Vault
 * @param   state   Contains a pointer to shared data for all AES GCM test cases.
 */
int test_vault_aead_aes_gcm_teardown(void** state)
{
  test_vault_aead_aes_gcm_shared_data_t* test_data = 0;

  /* ------------------- */
  /* Test Case Increment */
  /* ------------------- */

  test_data = (test_vault_aead_aes_gcm_shared_data_t*) *state;
  test_data->test_count++;

  return 0;
}

/**
 * @brief   Triggers AES GCM unit tests using Ockam Vault.
 * @return  Zero on success. Non-zero on failure.
 */
int test_vault_run_aead_aes_gcm(ockam_vault_t* vault, ockam_memory_t* memory, TEST_VAULT_AEAD_AES_GCM_KEY_e key)
{
  int                                   rc           = 0;
  char*                                 test_name    = 0;
  uint16_t                              i            = 0;
  uint8_t*                              cmocka_data  = 0;
  struct CMUnitTest*                    cmocka_tests = 0;
  test_vault_aead_aes_gcm_shared_data_t shared_data;

  ockam_error_t error = ockam_memory_alloc_zeroed(
    memory, (void**) &cmocka_data, TEST_VAULT_AEAD_AES_GCM_TEST_CASES * sizeof(struct CMUnitTest));
  if (ockam_error_has_error(&error)) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  shared_data.test_count     = 0;
  shared_data.test_count_max = TEST_VAULT_AEAD_AES_GCM_TEST_CASES;
  shared_data.vault          = vault;
  shared_data.memory         = memory;
  shared_data.test_key_type  = key;

  for (i = 0; i < TEST_VAULT_AEAD_AES_GCM_TEST_CASES; i++) {
    error = ockam_memory_alloc_zeroed(memory, (void**) &test_name, TEST_VAULT_AEAD_AES_GCM_NAME_SIZE);
    if (ockam_error_has_error(&error)) {
      rc = -1;
      goto exit_block;
    }

    snprintf(test_name, TEST_VAULT_AEAD_AES_GCM_NAME_SIZE, "AES GCM Test Case %02d", i);

    cmocka_tests->name          = test_name;
    cmocka_tests->test_func     = test_vault_aead_aes_gcm;
    cmocka_tests->setup_func    = 0;
    cmocka_tests->teardown_func = test_vault_aead_aes_gcm_teardown;
    cmocka_tests->initial_state = &shared_data;

    cmocka_tests++;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  rc = _cmocka_run_group_tests("AES-GCM", cmocka_tests, shared_data.test_count_max, 0, 0);

exit_block:
  return rc;
}
