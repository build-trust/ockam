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

#define TEST_VAULT_KEY_P256_TEST_CASES               1u
#define TEST_VAULT_KEY_CURVE25519_TEST_CASES         2u

#define TEST_VAULT_KEY_P256_SIZE                    64u
#define TEST_VAULT_KEY_CURVE25519_SIZE              32u

#define TEST_VAULT_PMS_SIZE                         32u


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
    uint8_t initiator_priv[TEST_VAULT_KEY_P256_SIZE];
    uint8_t initiator_pub[TEST_VAULT_KEY_P256_SIZE];
    uint8_t responder_priv[TEST_VAULT_KEY_P256_SIZE];
    uint8_t responder_pub[TEST_VAULT_KEY_P256_SIZE];
} TEST_VAULT_KEYS_P256_s;


/**
 *******************************************************************************
 * @struct  TEST_VAULT_KEYS_CURVE25519_s
 * @brief   Initiator and responder test keys on Curve25519
 *******************************************************************************
 */

typedef struct {
    uint8_t initiator_priv[TEST_VAULT_KEY_CURVE25519_SIZE];
    uint8_t initiator_pub[TEST_VAULT_KEY_CURVE25519_SIZE];
    uint8_t responder_priv[TEST_VAULT_KEY_CURVE25519_SIZE];
    uint8_t responder_pub[TEST_VAULT_KEY_CURVE25519_SIZE];
} TEST_VAULT_KEYS_CURVE25519_s;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_key_ecdh_print(OCKAM_LOG_e level, uint8_t test_case, char *p_str);


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
        }
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

void test_vault_key_ecdh(OCKAM_VAULT_EC_e ec, uint8_t load_keys)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    uint8_t i = 0;
    uint8_t j = 0;
    uint8_t test_cases = 0;
    uint32_t key_size = 0;
    int ret = 0;

    uint8_t *p_static_pub     = 0;
    uint8_t *p_ephemeral_pub  = 0;

    uint8_t pms_static[TEST_VAULT_PMS_SIZE];
    uint8_t pms_ephemeral[TEST_VAULT_PMS_SIZE];


    switch(ec) {                                                /* Configure the Key/ECDH tests based on the platform */
        case OCKAM_VAULT_EC_P256:                               /* being tested.                                      */
            test_cases = TEST_VAULT_KEY_P256_TEST_CASES;
            key_size = 64;
            break;

        case OCKAM_VAULT_EC_CURVE25519:
            test_cases = TEST_VAULT_KEY_CURVE25519_TEST_CASES;
            key_size = 32;
            break;

        default:
            break;
    }

    if(!load_keys) {                                            /* If the vault we're using doesn't support loading   */
        test_cases = 1;                                         /* private keys, just loop once and generate keys     */
    }

    err = ockam_mem_alloc((void**) &p_static_pub,               /* Grab memory for the static public key that is      */
                          key_size);                            /* pulled from Vault                                  */
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_FATAL,
                                  i,
                                  "Public Key Static Memory Allocation Fail");
        return;
    }

    err = ockam_mem_alloc((void**)&p_ephemeral_pub,             /* Grab memory for the ephemeral public key that is   */
                          key_size);                            /* pulled from Vault                                  */
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_FATAL,
                                  i,
                                  "Public Key Ephemeral Memory Allocation Fail");
        return;
    }


    /* -------------- */
    /* Test Case Loop */
    /* -------------- */

    for(i = 0; i < test_cases; i++) {

        uint8_t pms_invalid = 0;
        uint8_t *p_initiator_priv = 0;
        uint8_t *p_initiator_pub  = 0;
        uint8_t *p_responder_priv = 0;
        uint8_t *p_responder_pub  = 0;


        /* ------------------ */
        /* Key Write/Generate */
        /* ------------------ */

        if(load_keys) {
            if(ec == OCKAM_VAULT_EC_P256) {                     /* Grab the public and private keys for the test case */
                p_initiator_priv = &g_test_vault_keys_p256[i].initiator_priv[0];
                p_initiator_pub  = &g_test_vault_keys_p256[i].initiator_pub[0];
                p_responder_priv = &g_test_vault_keys_p256[i].responder_priv[0];
                p_responder_pub  = &g_test_vault_keys_p256[i].responder_pub[0];
            } else if(ec == OCKAM_VAULT_EC_CURVE25519) {
                p_initiator_priv = &(g_test_vault_keys_curve25519[i].initiator_priv[0]);
                p_initiator_pub  = &(g_test_vault_keys_curve25519[i].initiator_pub[0]);
                p_responder_priv = &(g_test_vault_keys_curve25519[i].responder_priv[0]);
                p_responder_pub  = &(g_test_vault_keys_curve25519[i].responder_pub[0]);
            }

            err = ockam_vault_key_write(OCKAM_VAULT_KEY_STATIC, /* Write the initiator key to the static slot         */
                                        p_initiator_priv, key_size);
            if(err != OCKAM_ERR_NONE) {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Static Key Write Failed");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Static Key Write Success");
            }
                                                                /* Write the responder key to the epehemral slot      */
            err = ockam_vault_key_write(OCKAM_VAULT_KEY_EPHEMERAL,
                                        p_responder_priv, key_size);
            if(err != OCKAM_ERR_NONE) {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Ephemeral Key Write Failed");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Ephemeral Key Write Success");
            }
        } else {                                                /* If the platform doesn't support writing keys, then */
            err = ockam_vault_key_gen(OCKAM_VAULT_KEY_STATIC);  /* generate a static key                              */
            if(err != OCKAM_ERR_NONE) {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Static Key Generate Failed");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Static Key Generate Success");
            }
                                                                /* Generate an ephemrmal key                          */
            err = ockam_vault_key_gen(OCKAM_VAULT_KEY_EPHEMERAL);
            if(err != OCKAM_ERR_NONE) {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Ephemeral Key Generate Failed");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Ephemeral Key Generate Success");
            }
        }


        /* ------------ */
        /* Key Retrival */
        /* ------------ */

        err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_STATIC,   /* Get the static public key                          */
                                      p_static_pub,
                                      key_size);
        if(err != OCKAM_ERR_NONE) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      i,
                                      "Get Static Public Key Failed");
        } else {
            test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                      i,
                                      "Get Static Public Key Success");
        }

        err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_EPHEMERAL,/* Get the ephemeral public key                       */
                                      p_ephemeral_pub,
                                      key_size);
        if(err != OCKAM_ERR_NONE) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      i,
                                      "Get Static Public Key Failed");
        } else {
            test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                      i,
                                      "Get Static Public Key Success");
        }
                                                                /* Only compare public keys to test cases if the  the */
        if(load_keys) {                                         /* key was not generated. Can't compare generated     */
            ret = memcmp(p_static_pub,                          /* since the result is unknown.                       */
                         p_initiator_pub,
                         key_size);
            if(!ret) {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Static Public Key Value Valid");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Static Public Key Value Invalid");
            }

            ret = memcmp(p_ephemeral_pub,                       /* Compare the generated public key to the test case  */
                         p_responder_pub,
                         key_size);
            if(!ret) {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "Ephemeral Public Key Value Valid");
            } else {
                test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                          i,
                                          "Ephemeral Public Key Value Invalid");
            }
        }


        /* ----------------- */
        /* ECDH Calculations */
        /* ----------------- */

        err = ockam_vault_ecdh(OCKAM_VAULT_KEY_STATIC,          /* Calculate ECDH with static private/ephemeral pub   */
                               p_ephemeral_pub,
                               key_size,
                               &pms_static[0],
                               TEST_VAULT_PMS_SIZE);
        if(err != OCKAM_ERR_NONE) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      i,
                                      "ECDH: Ephemeral Public/Static Private Failed");
        } else {
                test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                          i,
                                          "ECDH: Ephemeral Public/Static Private Success");
                test_vault_print_array(OCKAM_LOG_DEBUG,
                                       "KEY ECDH",
                                       "ECDH: Ephemeral Public/Static Private",
                                       &pms_static[0],
                                       key_size);
        }

        err = ockam_vault_ecdh(OCKAM_VAULT_KEY_EPHEMERAL,       /* Calculate ECDH with ephemeral private/static public*/
                               p_static_pub,
                               key_size,
                               &pms_ephemeral[0],
                               TEST_VAULT_PMS_SIZE);
        if(err != OCKAM_ERR_NONE) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      i,
                                      "ECDH: Static Public/Ephemeral Private Failed");
        } else {
            test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                      i,
                                      "ECDH: Static Public/Ephemeral Private Success");
            test_vault_print_array(OCKAM_LOG_DEBUG,
                                   "KEY ECDH",
                                   "ECDH: Static Public/Ephemeral Private",
                                   &pms_ephemeral[0],
                                   TEST_VAULT_PMS_SIZE);
        }

        for(j = 0; j < TEST_VAULT_PMS_SIZE; j++) {              /* Compare the PMS arrays byte by byte                */
            if(pms_static[j] != pms_ephemeral[j]) {
                pms_invalid = 1;
                break;
            }
        }

        if(pms_invalid) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      i,
                                      "PMS values do not match");
        } else {
            test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                      i,
                                      "PMS values match");
        }
    }
}


/**
 ********************************************************************************************************
 *                                          test_vault_key_ecdh_print()
 *
 * @brief   Central logging function for KEY ECDH tests
 *
 * @param   level       The log level for the specified message
 *
 * @param   test_case   The test case being logged
 *
 * @param   p_str       The message to print
 *
 ********************************************************************************************************
 */

void test_vault_key_ecdh_print(OCKAM_LOG_e level, uint8_t test_case, char *p_str)
{
    test_vault_print( level,
                     "KEY ECDH",
                      test_case,
                      p_str);
}

