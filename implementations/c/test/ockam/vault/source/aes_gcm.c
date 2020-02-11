/**
 ********************************************************************************************************
 * @file    aes_gcm.c
 * @brief   Common AES GCM test cases for Ockam Vault
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <cmocka.h>

#include <ockam/error.h>
#include <ockam/log.h>
#include <ockam/vault.h>
#include <ockam/memory.h>

#include <test_vault.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define TEST_VAULT_AES_GCM_TEST_CASES                2u         /*!< Total number of test cases to run                */
#define TEST_VAULT_AES_GCM_NAME_SIZE                32u         /*!< Size of the buffer to allocate for the test name */

#define TEST_VAULT_AES_GCM_KEY_SIZE                 16u         /*!< Use a 128-bit AES Key Size for the tests         */
#define TEST_VAULT_AES_GCM_TAG_SIZE                 16u         /*!< Size of the AES GCM Tag buffer. Always 16 bytes. */


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
 * @struct  TEST_VAULT_AES_GCM_DATA_s
 * @brief   Common AES GCM test data
 *******************************************************************************
 */

typedef struct {
    uint8_t *p_key;                                             /*!< AES GCM key for encryption/decryption            */
    uint8_t *p_aad;                                             /*!< AAD data for encryption/decryption               */
    uint32_t aad_size;                                          /*!< AAD data size                                    */
    uint8_t *p_iv;                                              /*!< IV data for encryption/decryption                */
    uint32_t iv_size;                                           /*!< IV data size                                     */
    uint8_t *p_tag;                                             /*!< Expected tag from encryption                     */
    uint8_t *p_plain_text;                                      /*!< Plain text data to be encrypted/decrypted        */
    uint8_t *p_encrypted_text;                                  /*!< Expected encrypted data                          */
    uint32_t text_size;                                         /*!< Size of the plain text and encrypted data        */
} TEST_VAULT_AES_GCM_DATA_s;


/**
 *******************************************************************************
 * @struct  TEST_VAULT_AES_GCM_SHARED_DATA_s
 * @brief   Shared test data for all unit tests
 *******************************************************************************
 */

typedef struct {
    uint16_t test_count;                                        /*!< Current unit test                                */
    uint16_t test_count_max;                                    /*!< Total number of unit tests                       */
} TEST_VAULT_AES_GCM_SHARED_DATA_s;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_aes_gcm(void **state);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

uint8_t g_aes_gcm_test1_key[] = {
    0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c,
    0x6d, 0x6a, 0x8f, 0x94, 0x67, 0x30, 0x83, 0x08
};

uint8_t g_aes_gcm_test1_aad[] = {
    0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
    0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef,
    0xab, 0xad, 0xda, 0xd2
};

uint8_t g_aes_gcm_test1_iv[] = {
    0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad,
    0xde, 0xca, 0xf8, 0x88
};

uint8_t g_aes_gcm_test1_tag[] = {
    0x5b, 0xc9, 0x4f, 0xbc, 0x32, 0x21, 0xa5, 0xdb,
    0x94, 0xfa, 0xe9, 0x5a, 0xe7, 0x12, 0x1a, 0x47
};

uint8_t g_aes_gcm_test2_tag[] = {
    0x34, 0x64, 0x34, 0xFD, 0x51, 0xD5, 0xCD, 0x0C,
    0x58, 0x87, 0xEC, 0x63, 0xE3, 0x9B, 0x90, 0x7A
};

uint8_t g_aes_gcm_test1_encrypted_text[] = {
    0xd9, 0x31, 0x32, 0x25, 0xf8, 0x84, 0x06, 0xe5,
    0xa5, 0x59, 0x09, 0xc5, 0xaf, 0xf5, 0x26, 0x9a,
    0x86, 0xa7, 0xa9, 0x53, 0x15, 0x34, 0xf7, 0xda,
    0x2e, 0x4c, 0x30, 0x3d, 0x8a, 0x31, 0x8a, 0x72,
    0x1c, 0x3c, 0x0c, 0x95, 0x95, 0x68, 0x09, 0x53,
    0x2f, 0xcf, 0x0e, 0x24, 0x49, 0xa6, 0xb5, 0x25,
    0xb1, 0x6a, 0xed, 0xf5, 0xaa, 0x0d, 0xe6, 0x57,
    0xba, 0x63, 0x7b, 0x39
};


uint8_t g_aes_gcm_test1_plain_text[] = {
    0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24,
    0x4b, 0x72, 0x21, 0xb7, 0x84, 0xd0, 0xd4, 0x9c,
    0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02, 0xa4, 0xe0,
    0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e,
    0x21, 0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c,
    0x7d, 0x8f, 0x6a, 0x5a, 0xac, 0x84, 0xaa, 0x05,
    0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac, 0x97,
    0x3d, 0x58, 0xe0, 0x91
};


TEST_VAULT_AES_GCM_DATA_s g_aes_gcm_data[TEST_VAULT_AES_GCM_TEST_CASES] =
{
    {
        &g_aes_gcm_test1_key[0],
        &g_aes_gcm_test1_aad[0],
        20,
        &g_aes_gcm_test1_iv[0],
        12,
        &g_aes_gcm_test1_tag[0],
        &g_aes_gcm_test1_encrypted_text[0],
        &g_aes_gcm_test1_plain_text[0],
        60
    },
    {
        &g_aes_gcm_test1_key[0],
        &g_aes_gcm_test1_aad[0],
        20,
        &g_aes_gcm_test1_iv[0],
        12,
        &g_aes_gcm_test2_tag[0],
        0,
        0,
        0
    },
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
 *                                          test_vault_aes_gcm()
 *
 * @brief   Run through encryption and decryption test cases using Ockam Vault
 *
 * @param   state   Contains a shared data pointer for common test data.
 *
 ********************************************************************************************************
 */

void test_vault_aes_gcm(void **state)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    TEST_VAULT_AES_GCM_SHARED_DATA_s *p_test_data = 0;

    uint8_t *p_aes_gcm_encrypt_hash = 0;
    uint8_t *p_aes_gcm_decrypt_data = 0;
    uint8_t aes_gcm_tag[TEST_VAULT_AES_GCM_TAG_SIZE];


    /* -------------------------- */
    /* Test Data and Verification */
    /* -------------------------- */

    p_test_data = (TEST_VAULT_AES_GCM_SHARED_DATA_s*) *state;

    if(p_test_data->test_count >= p_test_data->test_count_max) {
        fail_msg("Test count %d has exceeded max test count of %d",
                 p_test_data->test_count,
                 p_test_data->test_count_max);
    }

    /* ----------------- */
    /* Memory Allocation */
    /* ----------------- */

    if(g_aes_gcm_data[p_test_data->test_count].text_size > 0) {
        err = ockam_mem_alloc((void**) &p_aes_gcm_encrypt_hash,
                              g_aes_gcm_data[p_test_data->test_count].text_size);
        if(err != OCKAM_ERR_NONE) {
            fail_msg("Unable to allocate p_aes_gcm_encrypt_hash");
        }

        err = ockam_mem_alloc((void**) &p_aes_gcm_decrypt_data,
                              g_aes_gcm_data[p_test_data->test_count].text_size);
        if(err != OCKAM_ERR_NONE) {
            fail_msg("Unable to allocate p_aes_gcm_decrypt_hash");
        }
    }

    /* --------------- */
    /* AES GCM Encrypt */
    /* --------------- */

    err = ockam_vault_aes_gcm_encrypt(g_aes_gcm_data[p_test_data->test_count].p_key,
                                      TEST_VAULT_AES_GCM_KEY_SIZE,
                                      g_aes_gcm_data[p_test_data->test_count].p_iv,
                                      g_aes_gcm_data[p_test_data->test_count].iv_size,
                                      g_aes_gcm_data[p_test_data->test_count].p_aad,
                                      g_aes_gcm_data[p_test_data->test_count].aad_size,
                                      &aes_gcm_tag[0],
                                      TEST_VAULT_AES_GCM_TAG_SIZE,
                                      g_aes_gcm_data[p_test_data->test_count].p_plain_text,
                                      g_aes_gcm_data[p_test_data->test_count].text_size,
                                      p_aes_gcm_encrypt_hash,
                                      g_aes_gcm_data[p_test_data->test_count].text_size);
    assert_int_equal(err, OCKAM_ERR_NONE);

    assert_memory_equal(&aes_gcm_tag[0],                        /* Compare the computed tag with the expected tag     */
                        g_aes_gcm_data[p_test_data->test_count].p_tag,
                        TEST_VAULT_AES_GCM_TAG_SIZE);

    assert_memory_equal(p_aes_gcm_encrypt_hash,                 /* Compare the computed hash with the expected hash   */
                        g_aes_gcm_data[p_test_data->test_count].p_encrypted_text,
                        g_aes_gcm_data[p_test_data->test_count].text_size);

    /* --------------- */
    /* AES GCM Decrypt */
    /* --------------- */

    err = ockam_vault_aes_gcm_decrypt(g_aes_gcm_data[p_test_data->test_count].p_key,
                                      TEST_VAULT_AES_GCM_KEY_SIZE,
                                      g_aes_gcm_data[p_test_data->test_count].p_iv,
                                      g_aes_gcm_data[p_test_data->test_count].iv_size,
                                      g_aes_gcm_data[p_test_data->test_count].p_aad,
                                      g_aes_gcm_data[p_test_data->test_count].aad_size,
                                      g_aes_gcm_data[p_test_data->test_count].p_tag,
                                      TEST_VAULT_AES_GCM_TAG_SIZE,
                                      g_aes_gcm_data[p_test_data->test_count].p_encrypted_text,
                                      g_aes_gcm_data[p_test_data->test_count].text_size,
                                      p_aes_gcm_decrypt_data,
                                      g_aes_gcm_data[p_test_data->test_count].text_size);
    assert_int_equal(err, OCKAM_ERR_NONE);

    assert_memory_equal(p_aes_gcm_decrypt_data,                 /* Compare the computed hash with the expected hash   */
                        g_aes_gcm_data[p_test_data->test_count].p_plain_text,
                        g_aes_gcm_data[p_test_data->test_count].text_size);

    /* ----------- */
    /* Memory Free */
    /* ----------- */

    ockam_mem_free(p_aes_gcm_encrypt_hash);                     /* Ignore the error result. Some tests don't allocate */
    ockam_mem_free(p_aes_gcm_decrypt_data);                     /* memory which freeing results in an error.          */

    /* ------------------- */
    /* Test Case Increment */
    /* ------------------- */

    p_test_data->test_count++;
}


/**
 ********************************************************************************************************
 *                                          test_vault_run_aes_gcm()
 *
 * @brief   Triggers HKDF unit tests using Ockam Vault.
 *
 * @return  Zero on success. Non-zero on failure.
 *
 ********************************************************************************************************
 */

int test_vault_run_aes_gcm(void)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    int rc = 0;
    char *p_test_name = 0;
    uint16_t i = 0;
    uint8_t *p_cmocka_data = 0;
    struct CMUnitTest *p_cmocka_tests = 0;
    TEST_VAULT_AES_GCM_SHARED_DATA_s shared_data;


    do {
        err = ockam_mem_alloc((void**) &p_cmocka_data,          /* Allocate test structs based on total test cases    */
                              (TEST_VAULT_AES_GCM_TEST_CASES * sizeof(struct CMUnitTest)));
        if(err != OCKAM_ERR_NONE) {
            rc = -1;
            break;
        }

        p_cmocka_tests = (struct CMUnitTest*) p_cmocka_data;    /* Set the unit test pointer to the allocated data    */

        shared_data.test_count = 0;                             /* Set initial values for the test shared data        */
        shared_data.test_count_max = TEST_VAULT_AES_GCM_TEST_CASES;

        for(i = 0; i < TEST_VAULT_AES_GCM_TEST_CASES; i++) {    /* Configure a CMocka unit test for each test case    */

            err = ockam_mem_alloc((void**) &p_test_name,
                                  TEST_VAULT_AES_GCM_NAME_SIZE);
            if(err != OCKAM_ERR_NONE) {
                rc = -1;
                break;
            }

            snprintf(p_test_name,                               /* Set the individual test name based on test count   */
                     TEST_VAULT_AES_GCM_NAME_SIZE,
                     "AES GCM Test Case %02d",
                     i);

            p_cmocka_tests->name = p_test_name;                 /* Set the name, test function and shared data for    */
            p_cmocka_tests->test_func = test_vault_aes_gcm;     /* the CMocka unit test.                              */
            p_cmocka_tests->setup_func = 0;
            p_cmocka_tests->teardown_func = 0;
            p_cmocka_tests->initial_state = &shared_data;

            p_cmocka_tests++;                                   /* Bump the unit test pointer                         */
        }

        if(err != OCKAM_ERR_NONE) {                             /* Ensure there were no memory allocation errors      */
            rc = -1;
            break;
        }

        p_cmocka_tests = (struct CMUnitTest*) p_cmocka_data;    /* Reset CMocka pointer to the front of the data block*/

        rc = _cmocka_run_group_tests("AES-GCM",                 /* Run the AES-GCM unit tests                         */
                                     p_cmocka_tests,
                                     shared_data.test_count_max,
                                     0,
                                     0);
    } while(0);

    return rc;
}

