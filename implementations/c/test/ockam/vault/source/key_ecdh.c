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

void test_vault_key_ecdh(OCKAM_VAULT_EC_e ec)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    uint8_t i = 0;
    uint32_t key_size = 0;
    uint8_t *p_pub_key = 0;
    uint8_t *p_key_static = 0;
    uint8_t *p_key_ephemeral = 0;
    uint8_t pms_static[TEST_VAULT_PMS_SIZE];
    uint8_t pms_ephemeral[TEST_VAULT_PMS_SIZE];


    /* ----------- */
    /* Key Buffers */
    /* ----------- */

    switch(ec) {
        case OCKAM_VAULT_EC_P256:
            key_size = 64;
            break;

        case OCKAM_VAULT_EC_CURVE25519:
            key_size = 32;
            break;

        default:
            break;
    }

    err = ockam_mem_alloc(&p_pub_key,
                          (key_size * TOTAL_TEST_VAULT_PUB_KEY));
    if(err != OCKAM_ERR_NONE) {
        test_vault_key_ecdh_print(OCKAM_LOG_ERROR,
                                  "Unable to allocate key buffers");
        return;
    }

    p_key_static = p_pub_key + (key_size * TEST_VAULT_PUB_KEY_STATIC);
    p_key_ephemeral = p_pub_key + (key_size * TEST_VAULT_PUB_KEY_EPHEMERAL);

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
                                  key_size);
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
                               key_size);
    }

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_EPHEMERAL,    /* Get the ephemrmal public key                       */
                                  p_key_ephemeral,
                                  key_size);
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
                               key_size);
    }

    /* ----------------- */
    /* ECDH Calculations */
    /* ----------------- */

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_STATIC,              /* Calculate ECDH with static private/ephemeral pub   */
                           p_key_ephemeral,
                           key_size,
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
                               key_size);
    }

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_EPHEMERAL,          /* Calculate ECDH with static private/ephemeral pub    */
                           p_key_static,
                           key_size,
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
