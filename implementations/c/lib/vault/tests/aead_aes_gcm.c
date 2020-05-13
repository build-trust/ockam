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

#define TEST_VAULT_AEAD_AES_GCM_TEST_CASES 2u
#define TEST_VAULT_AEAD_AES_GCM_NAME_SIZE  32u
#define TEST_VAULT_AEAD_AES_GCM_KEY_SIZE   16u
#define TEST_VAULT_AEAD_AES_GCM_TAG_SIZE   16u

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
  uint16_t        test_count;
  uint16_t        test_count_max;
  ockam_vault_t*  vault;
  ockam_memory_t* memory;
} test_vault_aead_aes_gcm_shared_data_t;

void test_vault_aead_aes_gcm(void** state);
int  test_vault_aead_aes_gcm_teardown(void** state);

/* clang-format off */

uint8_t g_aead_aes_gcm_test_0_key[] =
{
  0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c,
  0x6d, 0x6a, 0x8f, 0x94, 0x67, 0x30, 0x83, 0x08
};

//feffe9928665731c6d6a8f9467308308

uint8_t g_aead_aes_gcm_test_1_key[] =
{
  0xc5, 0x02, 0x74, 0xde, 0x93, 0xe9, 0x96, 0xb6,
  0x61, 0xf1, 0xa6, 0xf1, 0xeb, 0x7d, 0xaa, 0x9d,
  0xda, 0xbf, 0x1d, 0xe2, 0x0a, 0x83, 0xd3, 0xbf,
  0xa6, 0xdb, 0xe3, 0xb9, 0x22, 0x02, 0x2a, 0x48
};

uint8_t g_aead_aes_gcm_test_0_ciphertext_and_tag[] =
{
  0xf8, 0x81, 0xf1, 0x29, 0x10, 0xdc, 0xe2, 0x77,
  0x2e, 0xc3, 0xf6, 0x28, 0x84, 0x5f, 0xf9, 0x47,
  0x50, 0x78, 0xdb, 0x0f, 0x96, 0x70, 0x05, 0x5a,
  0x1a, 0xd5, 0xc8, 0xbf, 0x65, 0x86, 0x3b, 0x70
};

uint8_t g_aead_aes_gcm_test_1_ciphertext_and_tag[] =
{
  0xd2, 0x16, 0xa7, 0xbc, 0x0c, 0xac, 0x23, 0xeb,
  0xba, 0x80, 0xb2, 0x58, 0x20, 0xf4, 0x58, 0x45,
  0x30, 0xb2, 0x7b, 0x53, 0x3c, 0x52, 0x84, 0x81,
  0xb3, 0xf6, 0x27, 0x27, 0x4d, 0xfc, 0xa1, 0xc3
};

uint8_t g_aead_aes_gcm_test_aad[] =
{
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
  0xab, 0xad, 0xda, 0xd2
};

//feedfacedeadbeeffeedfacedeadbeefabaddad2

uint8_t g_aead_aes_gcm_test_plaintext[] =
{
  0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
  0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F
};

//000102030405060708090A0B0C0D0E0F

test_vault_aead_aes_gcm_data_t g_aead_aes_gcm_data[TEST_VAULT_AEAD_AES_GCM_TEST_CASES] =
{
  {
    &g_aead_aes_gcm_test_0_key[0],
    16,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext[0],
    &g_aead_aes_gcm_test_0_ciphertext_and_tag[0],
    16,
  },
  {
    &g_aead_aes_gcm_test_1_key[0],
    32,
    &g_aead_aes_gcm_test_aad[0],
    20,
    0xCAFE,
    &g_aead_aes_gcm_test_plaintext[0],
    &g_aead_aes_gcm_test_1_ciphertext_and_tag[0],
    16
  },
};

/* clang-format on */

/**
 * @brief   Run through encryption and decryption test cases using Ockam Vault
 * @param   state   Contains a shared data pointer for common test data.
 */
void test_vault_aead_aes_gcm(void** state)
{
  ockam_error_t                          error                          = OCKAM_ERROR_NONE;
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

  /* ----------------- */
  /* Memory Allocation */
  /* ----------------- */

  if (g_aead_aes_gcm_data[test_data->test_count].text_size > 0) {
    aead_aes_gcm_encrypt_hash_size =
      g_aead_aes_gcm_data[test_data->test_count].text_size + TEST_VAULT_AEAD_AES_GCM_TAG_SIZE;

    error =
      ockam_memory_alloc_zeroed(test_data->memory, (void**) &aead_aes_gcm_encrypt_hash, aead_aes_gcm_encrypt_hash_size);
    if (error != OCKAM_ERROR_NONE) { fail_msg("Unable to alloc aead_aes_gcm_encrypt_hash"); }

    aead_aes_gcm_decrypt_data_size = g_aead_aes_gcm_data[test_data->test_count].text_size;

    error =
      ockam_memory_alloc_zeroed(test_data->memory, (void**) &aead_aes_gcm_decrypt_data, aead_aes_gcm_decrypt_data_size);
    if (error != OCKAM_ERROR_NONE) { fail_msg("Unable to alloc aead_aes_gcm_decrypt_data"); }
  }

  /* ------- */
  /* AES Key */
  /* ------- */

  attributes.purpose     = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
  attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  attributes.type        = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  attributes.length      = g_aead_aes_gcm_data[test_data->test_count].key_size;

  error = ockam_vault_secret_import(test_data->vault,
                                    &key_secret,
                                    &attributes,
                                    g_aead_aes_gcm_data[test_data->test_count].key,
                                    g_aead_aes_gcm_data[test_data->test_count].key_size);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  if (attributes.length == OCKAM_VAULT_AES128_KEY_LENGTH) {
    error = ockam_vault_secret_type_set(test_data->vault, &key_secret, OCKAM_VAULT_SECRET_TYPE_AES128_KEY);
    assert_int_equal(error, OCKAM_ERROR_NONE);
  } else if (attributes.length == OCKAM_VAULT_AES256_KEY_LENGTH) {
    error = ockam_vault_secret_type_set(test_data->vault, &key_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
    assert_int_equal(error, OCKAM_ERROR_NONE);
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
  assert_int_equal(error, OCKAM_ERROR_NONE);
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
  assert_int_equal(error, OCKAM_ERROR_NONE);
  assert_int_equal(length, aead_aes_gcm_decrypt_data_size);

  assert_memory_equal(
    aead_aes_gcm_decrypt_data, g_aead_aes_gcm_data[test_data->test_count].plaintext, aead_aes_gcm_decrypt_data_size);

  /* ----------- */
  /* Memory Free */
  /* ----------- */

  // TODO this will not be freed on an error
  ockam_memory_free(test_data->memory, aead_aes_gcm_encrypt_hash, aead_aes_gcm_encrypt_hash_size);
  ockam_memory_free(test_data->memory, aead_aes_gcm_decrypt_data, g_aead_aes_gcm_data[test_data->test_count].text_size);
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
int test_vault_run_aead_aes_gcm(ockam_vault_t* vault, ockam_memory_t* memory)
{
  ockam_error_t                         error        = OCKAM_ERROR_NONE;
  int                                   rc           = 0;
  char*                                 test_name    = 0;
  uint16_t                              i            = 0;
  uint8_t*                              cmocka_data  = 0;
  struct CMUnitTest*                    cmocka_tests = 0;
  test_vault_aead_aes_gcm_shared_data_t shared_data;

  error = ockam_memory_alloc_zeroed(
    memory, (void**) &cmocka_data, TEST_VAULT_AEAD_AES_GCM_TEST_CASES * sizeof(struct CMUnitTest));
  if (error != OCKAM_ERROR_NONE) {
    rc = -1;
    goto exit_block;
  }

  cmocka_tests = (struct CMUnitTest*) cmocka_data;

  shared_data.test_count     = 0;
  shared_data.test_count_max = TEST_VAULT_AEAD_AES_GCM_TEST_CASES;
  shared_data.vault          = vault;
  shared_data.memory         = memory;

  for (i = 0; i < TEST_VAULT_AEAD_AES_GCM_TEST_CASES; i++) {
    error = ockam_memory_alloc_zeroed(memory, (void**) &test_name, TEST_VAULT_AEAD_AES_GCM_NAME_SIZE);
    if (error != OCKAM_ERROR_NONE) {
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
