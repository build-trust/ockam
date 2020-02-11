/**
 ********************************************************************************************************
 * @file    key_ecdh.c
 * @brief   Ockam Vault common tests for key generation and ECDH
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

#define TEST_VAULT_KEY_NAME_SIZE                    32u         /*!< Size of the buffer to allocate for the test name */

#define TEST_VAULT_KEY_P256_TEST_CASES               1u         /*!< Total number of P-256 test cases to run          */
#define TEST_VAULT_KEY_CURVE25519_TEST_CASES         2u         /*!< Total number of Curve25519 test cases to run     */

#define TEST_VAULT_KEY_P256_SIZE                    64u         /*!< P-256 keys use 64 bytes                          */
#define TEST_VAULT_KEY_CURVE25519_SIZE              32u         /*!< Curve25519 keys use 32 bytes                     */

#define TEST_VAULT_SS_SIZE                          32u         /* Shared secretes are 32 bytes for both curves       */


/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @enum    TEST_VAULT_PUB_KEY_e
 * @brief   List of public keys to manage
 *******************************************************************************
 */

typedef enum {
    TEST_VAULT_PUB_KEY_STATIC   = 0,                            /*!< Static key in vault                              */
    TEST_VAULT_PUB_KEY_EPHEMERAL,                               /*!< Ephemeral key in vault                           */
    TOTAL_TEST_VAULT_PUB_KEY                                    /*!< Total number of keys handled                     */
} TEST_VAULT_PUB_KEY_e;


/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @struct  TEST_VAULT_KEYS_P256_s
 * @brief   Initiator and responder test keys on P256
 *******************************************************************************
 */

typedef struct {
    uint8_t initiator_priv[TEST_VAULT_KEY_P256_SIZE];           /*!< Initiator P-256 private key data buffer          */
    uint8_t initiator_pub[TEST_VAULT_KEY_P256_SIZE];            /*!< Initiator P-256 public key data buffer           */
    uint8_t responder_priv[TEST_VAULT_KEY_P256_SIZE];           /*!< Responder P-256 private key data buffer          */
    uint8_t responder_pub[TEST_VAULT_KEY_P256_SIZE];            /*!< Responder P-256 public key data buffer           */
} TEST_VAULT_KEYS_P256_s;


/**
 *******************************************************************************
 * @struct  TEST_VAULT_KEYS_CURVE25519_s
 * @brief   Initiator and responder test keys on Curve25519
 *******************************************************************************
 */

typedef struct {
    uint8_t initiator_priv[TEST_VAULT_KEY_CURVE25519_SIZE];     /*!< Initiator Curve25519 private key data buffer     */
    uint8_t initiator_pub[TEST_VAULT_KEY_CURVE25519_SIZE];      /*!< Initiator Curve25519 public key data buffer      */
    uint8_t responder_priv[TEST_VAULT_KEY_CURVE25519_SIZE];     /*!< Responder Curve25519 private key data buffer     */
    uint8_t responder_pub[TEST_VAULT_KEY_CURVE25519_SIZE];      /*!< Responder Curve25519 public key data buffer      */
    uint8_t shared_secret[TEST_VAULT_KEY_CURVE25519_SIZE];      /*!< Curve25519 expected shared secret data           */
} TEST_VAULT_KEYS_CURVE25519_s;


/**
 *******************************************************************************
 * @struct  TEST_VAULT_KEY_SHARED_DATA_s
 * @brief   Global test data for each test run
 *******************************************************************************
 */

typedef struct {
    uint16_t test_count;                                        /*!< Current unit test                                */
    uint16_t test_count_max;                                    /*!< Total number of unit tests                       */
    uint8_t load_keys;                                          /*!< 0=generate private key, 1=load private key       */
    uint8_t key_size;                                           /*!< Key size being used in the test                  */
    OCKAM_VAULT_EC_e ec;                                        /*!< Curve type being used in the test                */
} TEST_VAULT_KEY_SHARED_DATA_s;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_key_ecdh(void **state);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */


TEST_VAULT_KEYS_P256_s g_test_vault_keys_p256[TEST_VAULT_KEY_P256_TEST_CASES] =
{
    {
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,     /* Case 0: Initiator Private Key                      */
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        },
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,     /* Case 0: Initiator Public Key                       */
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        },
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,     /* Case 0: Responder Private Key                      */
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        },
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,     /* Case 0: Responder Public Key                       */
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        }
    }
};

TEST_VAULT_KEYS_CURVE25519_s g_test_vault_keys_curve25519[TEST_VAULT_KEY_CURVE25519_TEST_CASES] =
{
    {
        {
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,     /* Case 0: Initiator Private Key                      */
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
            0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f
        },
        {
            0x8f, 0x40, 0xc5, 0xad, 0xb6, 0x8f, 0x25, 0x62,     /* Case 0: Initiator Public Key                       */
            0x4a, 0xe5, 0xb2, 0x14, 0xea, 0x76, 0x7a, 0x6e,
            0xc9, 0x4d, 0x82, 0x9d, 0x3d, 0x7b, 0x5e, 0x1a,
            0xd1, 0xba, 0x6f, 0x3e, 0x21, 0x38, 0x28, 0x5f
        },
        {
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,     /* Case 0: Responder Private Key                      */
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
        },
        {
            0x07, 0xa3, 0x7c, 0xbc, 0x14, 0x20, 0x93, 0xc8,     /* Case 0: Responder Public Key                       */
            0xb7, 0x55, 0xdc, 0x1b, 0x10, 0xe8, 0x6c, 0xb4,
            0x26, 0x37, 0x4a, 0xd1, 0x6a, 0xa8, 0x53, 0xed,
            0x0b, 0xdf, 0xc0, 0xb2, 0xb8, 0x6d, 0x1c, 0x7c
        },
        {
            0x42, 0x74, 0xA3, 0x2E, 0x95, 0x3A, 0xCB, 0x83,     /* Case 0: Expected Shared Secret Value               */
            0x14, 0xD0, 0xF0, 0x9B, 0xCB, 0xCB, 0x51, 0x93,
            0xC5, 0xEF, 0x79, 0x9D, 0xDC, 0xD0, 0x03, 0x6F,
            0x8C, 0x46, 0x82, 0xE5, 0x80, 0x1D, 0xAC, 0x73
        }
    },
    {
        {
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,     /* Case 1: Initiator Private Key                      */
            0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
        },
        {
            0x35, 0x80, 0x72, 0xd6, 0x36, 0x58, 0x80, 0xd1,     /* Case 1: Initiator Public Key                       */
            0xae, 0xea, 0x32, 0x9a, 0xdf, 0x91, 0x21, 0x38,
            0x38, 0x51, 0xed, 0x21, 0xa2, 0x8e, 0x3b, 0x75,
            0xe9, 0x65, 0xd0, 0xd2, 0xcd, 0x16, 0x62, 0x54
        },
        {
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,     /* Case 1: Responder Private Key                      */
            0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
            0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
            0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60
        },
        {
            0x64, 0xb1, 0x01, 0xb1, 0xd0, 0xbe, 0x5a, 0x87,     /* Case 1: Responder Public Key                       */
            0x04, 0xbd, 0x07, 0x8f, 0x98, 0x95, 0x00, 0x1f,
            0xc0, 0x3e, 0x8e, 0x9f, 0x95, 0x22, 0xf1, 0x88,
            0xdd, 0x12, 0x8d, 0x98, 0x46, 0xd4, 0x84, 0x66
        },
        {
            0x37, 0xE0, 0xE7, 0xDA, 0xAC, 0xBD, 0x6B, 0xFB,     /* Case 1: Expected Shared Secret Value               */
            0xF6, 0x69, 0xA8, 0x46, 0x19, 0x6F, 0xD4, 0x4D,
            0x1C, 0x87, 0x45, 0xD3, 0x3F, 0x2B, 0xE4, 0x2E,
            0x31, 0xD4, 0x67, 0x41, 0x99, 0xAD, 0x00, 0x5E
        }
    },
};

const char g_test_vault_p256_name[] = "P-256: ";                /* Global strings for unit test printing              */
const char g_test_vault_curve25519_name[] = "Curve25519: ";


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
 *                                          test_vault_key_ecdh()
 *
 * @brief   Main unit test for Key/ECDH. Tests private key write/generate, public key retrieval, and
 *          ECDH. In cases where private keys were written to the device, public key data and shared
 *          secrets are validated against known values.
 *
 * @param   state   Contains the shared test data used in all Key/ECDH unit tests.
 *
 ********************************************************************************************************
 */

void test_vault_key_ecdh(void **state)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    TEST_VAULT_KEY_SHARED_DATA_s *p_test_data = 0;

    uint8_t *p_static_pub     = 0;
    uint8_t *p_ephemeral_pub  = 0;
    uint8_t *p_initiator_priv = 0;
    uint8_t *p_initiator_pub  = 0;
    uint8_t *p_responder_priv = 0;
    uint8_t *p_responder_pub  = 0;
    uint8_t *p_shared_secret  = 0;

    uint8_t ss_static[TEST_VAULT_SS_SIZE];
    uint8_t ss_ephemeral[TEST_VAULT_SS_SIZE];


    /* -------------------------- */
    /* Test Data and Verification */
    /* -------------------------- */

    p_test_data = (TEST_VAULT_KEY_SHARED_DATA_s*) *state;       /* Always get the shared test data first              */

    if(p_test_data->test_count >= p_test_data->test_count_max) {/* Ensure the test count has not exceeded the amount  */
        fail_msg("Test count %d has exceeded max tests of %d",  /* of available test data.                            */
                 p_test_data->test_count,
                 p_test_data->test_count_max);
    }

    /* ----------------- */
    /* Memory allocation */
    /* ----------------- */

    err = ockam_mem_alloc((void**) &p_static_pub,               /* Grab memory for the static public key that is      */
                          p_test_data->key_size);               /* pulled from Vault                                  */
    if(err != OCKAM_ERR_NONE) {
        fail_msg("Unable to allocate p_static_pub");
    }

    err = ockam_mem_alloc((void**)&p_ephemeral_pub,             /* Grab memory for the ephemeral public key that is   */
                          p_test_data->key_size);               /* pulled from Vault                                  */
    if(err != OCKAM_ERR_NONE) {
        fail_msg("Unable to allocate p_ephemeral_pub");
    }

    /* ------------------ */
    /* Key Write/Generate */
    /* ------------------ */

    if(p_test_data->load_keys) {
        if(p_test_data->ec == OCKAM_VAULT_EC_P256) {            /* Grab the public and private keys for the test case */
            p_initiator_priv = &g_test_vault_keys_p256[p_test_data->test_count].initiator_priv[0];
            p_initiator_pub  = &g_test_vault_keys_p256[p_test_data->test_count].initiator_pub[0];
            p_responder_priv = &g_test_vault_keys_p256[p_test_data->test_count].responder_priv[0];
            p_responder_pub  = &g_test_vault_keys_p256[p_test_data->test_count].responder_pub[0];
        } else if(p_test_data->ec == OCKAM_VAULT_EC_CURVE25519) {
            p_initiator_priv = &(g_test_vault_keys_curve25519[p_test_data->test_count].initiator_priv[0]);
            p_initiator_pub  = &(g_test_vault_keys_curve25519[p_test_data->test_count].initiator_pub[0]);
            p_responder_priv = &(g_test_vault_keys_curve25519[p_test_data->test_count].responder_priv[0]);
            p_responder_pub  = &(g_test_vault_keys_curve25519[p_test_data->test_count].responder_pub[0]);
            p_shared_secret  = &(g_test_vault_keys_curve25519[p_test_data->test_count].shared_secret[0]);
        }

        err = ockam_vault_key_write(OCKAM_VAULT_KEY_STATIC,     /* Write the initiator key to the static slot         */
                                    p_initiator_priv,
                                    p_test_data->key_size);
        assert_int_equal(err, OCKAM_ERR_NONE);
                                                                /* Write the responder key to the epehemral slot      */
        err = ockam_vault_key_write(OCKAM_VAULT_KEY_EPHEMERAL,
                                    p_responder_priv,
                                    p_test_data->key_size);
        assert_int_equal(err, OCKAM_ERR_NONE);
    } else {                                                    /* If the platform doesn't support writing keys, then */
        err = ockam_vault_key_gen(OCKAM_VAULT_KEY_STATIC);      /* generate a static key                              */
        assert_int_equal(err, OCKAM_ERR_NONE);
                                                                /* Generate an ephemrmal key                          */
        err = ockam_vault_key_gen(OCKAM_VAULT_KEY_EPHEMERAL);
        assert_int_equal(err, OCKAM_ERR_NONE);
    }

    /* ------------ */
    /* Key Retrival */
    /* ------------ */

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_STATIC,       /* Get the static public key                          */
                                  p_static_pub,
                                  p_test_data->key_size);
    assert_int_equal(err, OCKAM_ERR_NONE);

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_EPHEMERAL,    /* Get the ephemeral public key                       */
                                  p_ephemeral_pub,
                                  p_test_data->key_size);
    assert_int_equal(err, OCKAM_ERR_NONE);
                                                                /* Only compare public keys to test cases if the  the */
    if(p_test_data->load_keys) {                                /* key was not generated. Can't compare generated     */
        assert_memory_equal(p_static_pub,                       /* since the result is unknown.                       */
                            p_initiator_pub,
                            p_test_data->key_size);

        assert_memory_equal(p_ephemeral_pub,                    /* Compare the generated public key to the test case  */
                            p_responder_pub,
                            p_test_data->key_size);
    }

    /* ----------------- */
    /* ECDH Calculations */
    /* ----------------- */

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_STATIC,              /* Calculate ECDH with static private/ephemeral pub   */
                           p_ephemeral_pub,
                           p_test_data->key_size,
                           &ss_static[0],
                           TEST_VAULT_SS_SIZE);
    assert_int_equal(err, OCKAM_ERR_NONE);

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_EPHEMERAL,           /* Calculate ECDH with ephemeral private/static public*/
                           p_static_pub,
                           p_test_data->key_size,
                           &ss_ephemeral[0],
                           TEST_VAULT_SS_SIZE);
    assert_int_equal(err, OCKAM_ERR_NONE);

    assert_memory_equal(&ss_static[0],                          /* Compare the shared secert arrays                   */
                        &ss_ephemeral[0],
                        TEST_VAULT_SS_SIZE);
    if(p_test_data->load_keys) {
        assert_memory_equal(&ss_static[0],                      /* If the computed shared secrets match, validate it  */
                            p_shared_secret,
                            TEST_VAULT_SS_SIZE);
    }

    /* ----------- */
    /* Memory free */
    /* ----------- */

    err = ockam_mem_free((void*) p_static_pub);                 /* Free the allocated public keys                     */
    assert_int_equal(err, OCKAM_ERR_NONE);
    err = ockam_mem_free((void*) p_ephemeral_pub);
    assert_int_equal(err, OCKAM_ERR_NONE);

    /* -------------------- */
    /* Test Count Increment */
    /* -------------------- */

    p_test_data->test_count++;                                  /* Increment the shared test count for the next run   */
}


/**
 ********************************************************************************************************
 *                                          test_vault_key_ecdh_print()
 *
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
 *
 ********************************************************************************************************
 */

int test_vault_run_key_ecdh(OCKAM_VAULT_EC_e ec, uint8_t load_keys)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    int rc = 0;

    uint8_t i = 0;
    char *p_name = 0;
    char *p_test_name = 0;
    uint8_t *p_cmocka_data = 0;
    struct CMUnitTest *p_cmocka_tests = 0;
    TEST_VAULT_KEY_SHARED_DATA_s test_data = {0};


    do {
        test_data.ec = ec;                                      /* Save the input parameters to be processed in the   */
        test_data.load_keys = load_keys;                        /* group setup stage.                                 */
        test_data.test_count = 0;

        if(ec == OCKAM_VAULT_EC_P256) {                         /* Configure test count and key size based on curve   */
            test_data.test_count_max= TEST_VAULT_KEY_P256_TEST_CASES;
            test_data.key_size = 64;
            p_name = (char*) &g_test_vault_p256_name[0];
        } else if(ec == OCKAM_VAULT_EC_CURVE25519) {
            test_data.test_count_max = TEST_VAULT_KEY_CURVE25519_TEST_CASES;
            test_data.key_size = 32;
            p_name = (char*) &g_test_vault_curve25519_name[0];
        } else {
            rc = -1;
            break;
        }

        err = ockam_mem_alloc((void**) &p_cmocka_data,          /* Allocate a CMUnitTest struct for all test cases    */
                              test_data.test_count_max * sizeof(struct CMUnitTest));
        if(err != OCKAM_ERR_NONE) {
            rc = -1;
            break;
        }

        p_cmocka_tests = (struct CMUnitTest*) p_cmocka_data;    /* Set the unit test pointer to the allocated data    */

        for(i = 0; i < test_data.test_count_max; i++) {         /* Add all test cases to the group structure          */
            err = ockam_mem_alloc((void**) &p_test_name,
                                  TEST_VAULT_KEY_NAME_SIZE);
            if(err != OCKAM_ERR_NONE) {
                break;
            }

            snprintf(p_test_name,                               /* Set the test name depending on the curve type and  */
                     TEST_VAULT_KEY_NAME_SIZE,                  /* the loop value.                                    */
                     "%s Test Case %02d",
                     p_name,
                     i);

            p_cmocka_tests->name = p_test_name;                 /* Set the common unit test function and the common   */
            p_cmocka_tests->test_func = test_vault_key_ecdh;    /* test data. Setup and teardown are not used.        */
            p_cmocka_tests->setup_func = 0;
            p_cmocka_tests->teardown_func = 0;
            p_cmocka_tests->initial_state = &test_data;

            p_cmocka_tests++;                                   /* Increment to the next unit test data slot          */
        }

        if(err != OCKAM_ERR_NONE) {                             /* Ensure there were no errors during the individual  */
            rc = -1;                                            /* unit test configuration.                           */
            break;
        }

        p_cmocka_tests = (struct CMUnitTest*) p_cmocka_data;    /* Reset the unit test pointer to the allocated data  */

        rc = _cmocka_run_group_tests("KEY_ECDH",                /* Run the Key/ECDH unit tests                        */
                                     p_cmocka_tests,
                                     test_data.test_count_max,
                                     0,
                                     0);
    } while(0);

    return rc;
}

