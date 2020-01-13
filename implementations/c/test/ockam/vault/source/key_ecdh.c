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

#include <test_vault.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define TEST_VAULT_PMS_SIZE                         32u
#define TEST_VAULT_PUB_KEY_SIZE                     64u


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

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_key_ecdh_print(OCKAM_LOG_e level, char *p_str);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

uint8_t g_pub_key[TEST_VAULT_PUB_KEY_SIZE * TOTAL_TEST_VAULT_PUB_KEY];


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

void test_vault_key_ecdh()
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    uint8_t i = 0;
    uint8_t pms_static[TEST_VAULT_PMS_SIZE];
    uint8_t pms_ephemeral[TEST_VAULT_PMS_SIZE];

    uint8_t *p_key_static = &g_pub_key[TEST_VAULT_PUB_KEY_STATIC * TEST_VAULT_PUB_KEY_SIZE];
    uint8_t *p_key_ephemeral = &g_pub_key[TEST_VAULT_PUB_KEY_EPHEMERAL * TEST_VAULT_PUB_KEY_SIZE];


    /* -------------- */
    /* Key Generation */
    /* -------------- */

    err = ockam_vault_key_gen(OCKAM_VAULT_KEY_STATIC);          /* Generate a static key                              */
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "Static Key Generate Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "Static Key Generate Success");
    }

    err = ockam_vault_key_gen(OCKAM_VAULT_KEY_EPHEMERAL);       /* Generate an ephemrmal key                          */
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "Ephemeral Key Generate Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "Ephemeral Key Generate Success");
    }

    /* ------------ */
    /* Key Retrival */
    /* ------------ */

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_STATIC,       /* Get the static public key                          */
                                  p_key_static,
                                  TEST_VAULT_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "Get Static Public Key Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "Get Static Public Key Success");
        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "KEY ECDH",
                               "Public Static Key",
                               p_key_static,
                               TEST_VAULT_PUB_KEY_SIZE);
    }

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_EPHEMERAL,    /* Get the ephemrmal public key                       */
                                  p_key_ephemeral,
                                  TEST_VAULT_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "Get Ephemeral Public Key Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "Get Ephemeral Public Key Success");
        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "KEY ECDH",
                               "Public Ephemeral Key",
                               p_key_ephemeral,
                               TEST_VAULT_PUB_KEY_SIZE);
    }

    /* ----------------- */
    /* ECDH Calculations */
    /* ----------------- */

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_STATIC,              /* Calculate ECDH with static private/ephemeral pub   */
                           p_key_ephemeral,
                           TEST_VAULT_PUB_KEY_SIZE,
                           &pms_static[0],
                           TEST_VAULT_PMS_SIZE);
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "ECDH: Ephemeral Public/Static Private Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "ECDH: Ephemeral Public/Static Private Success");
        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "KEY ECDH",
                               "ECDH: Ephemeral Public/Static Private",
                               p_key_ephemeral,
                               TEST_VAULT_PUB_KEY_SIZE);
    }

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_EPHEMERAL,          /* Calculate ECDH with static private/ephemeral pub    */
                           p_key_static,
                           TEST_VAULT_PUB_KEY_SIZE,
                           &pms_ephemeral[0],
                           TEST_VAULT_PMS_SIZE);
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "ECDH: Static Public/Ephemeral Private Failed");
    } else {
        test_vault_key_ecdh_print(OCKAM_LOG_INFO,
                                  "ECDH: Static Public/Ephemeral Private Success");
        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "KEY ECDH",
                               "ECDH: Static Public/Ephemeral Private",
                               &pms_ephemeral[0],
                               TEST_VAULT_PMS_SIZE);
    }

    for(i = 0; i < TEST_VAULT_PMS_SIZE; i++) {
        if(pms_static[i] != pms_ephemeral[i]) {
            test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                      "PMS values do not match");
            break;
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
 * @param   p_str       The message to print
 *
 ********************************************************************************************************
 */

void test_vault_key_ecdh_print(OCKAM_LOG_e level, char *p_str)
{
    test_vault_print( level,
                     "KEY ECDH",
                      TEST_VAULT_NO_TEST_CASE,
                      p_str);
}
