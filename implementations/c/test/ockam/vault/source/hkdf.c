/**
 ********************************************************************************************************
 * @file    hkdf.c
 * @brief   Common HKDF test functions for Ockam Vault
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

#define TEST_VAULT_HKDF_CASES                       1u


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
 * @struct  TEST_VAULT_HKDF_DATA_s
 * @brief
 *******************************************************************************
 */
typedef struct {
    uint8_t *p_shared_secret;                                   /*!< Shared secret value to use for HKDF              */
    uint32_t shared_secret_size;                                /*!< Size of the shared secret value                  */
    uint8_t *p_salt;                                            /*!< Salt value for HKDF. Must fit into HW slot       */
    uint32_t salt_size;                                         /*!< Size of the salt value                           */
    uint8_t *p_info;                                            /*!< Optional info data for HKDF                      */
    uint32_t info_size;                                         /*!< Size of the info value                           */
    uint8_t *p_output;                                          /*!< Expected output from HKDF operation              */
    uint32_t output_size;                                       /*!< Size of the output to generate                   */
} TEST_VAULT_HKDF_DATA_s;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_hkdf_print(OCKAM_LOG_e level, uint32_t test_case, char *p_str);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

uint8_t g_hkdf_test_1_shared_secret[] = {
    0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
    0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
    0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b
};

uint8_t g_hkdf_test_1_salt[] = {
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0a, 0x0b, 0x0c
};

uint8_t g_hkdf_test_1_info[] = {
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
    0xf8, 0xf9
};


uint8_t g_hkdf_test_1_output[] = {
    0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a,
    0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36, 0x2f, 0x2a,
    0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c,
    0x5d, 0xb0, 0x2d, 0x56, 0xec, 0xc4, 0xc5, 0xbf,
    0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18,
    0x58, 0x65
};


TEST_VAULT_HKDF_DATA_s g_hkdf_data[TEST_VAULT_HKDF_CASES] =
{
    {
        &g_hkdf_test_1_shared_secret[0],
        22,
        &g_hkdf_test_1_salt[0],
        13,
        &g_hkdf_test_1_info[0],
        10,
        &g_hkdf_test_1_output[0],
        42
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
 *                                          test_vault_hkdf()
 *
 * @brief   Common test functions for HKDF using Ockam Vault
 *
 ********************************************************************************************************
 */

void test_vault_hkdf()
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    uint32_t i = 0;
    int hkdf_cmp = 0;


    for(i = 0; i < TEST_VAULT_HKDF_CASES; i++) {

        uint8_t hkdf_key[g_hkdf_data[i].output_size];

        err = ockam_vault_hkdf( g_hkdf_data[i].p_salt,          /* Calculate HKDF using test vectors                  */
                                g_hkdf_data[i].salt_size,
                                g_hkdf_data[i].p_shared_secret,
                                g_hkdf_data[i].shared_secret_size,
                                g_hkdf_data[i].p_info,
                                g_hkdf_data[i].info_size,
                               &hkdf_key[0],
                                g_hkdf_data[i].output_size);
        if(err != OCKAM_ERR_NONE) {
            test_vault_hkdf_print(OCKAM_LOG_ERROR,
                                  i,
                                  "HKDF Operation Failed");
        } else {
            hkdf_cmp = memcmp(&hkdf_key[0],
                               g_hkdf_data[i].p_output,
                               g_hkdf_data[i].output_size);
            if(hkdf_cmp != 0) {
                test_vault_hkdf_print(OCKAM_LOG_ERROR,
                                      i,
                                      "HKDF Calculation Invalid");

            } else {
                test_vault_hkdf_print(OCKAM_LOG_INFO,
                                      i,
                                      "HKDF Calculation Valid");
            }

            test_vault_print_array(OCKAM_LOG_DEBUG,
                                   "HKDF",
                                   "Calculated Key",
                                   &hkdf_key[0],
                                    g_hkdf_data[i].output_size);

            test_vault_print_array(OCKAM_LOG_DEBUG,
                                   "HKDF",
                                   "Expected Key",
                                   g_hkdf_data[i].p_output,
                                   g_hkdf_data[i].output_size);
        }
    }
}


/**
 ********************************************************************************************************
 *                                          test_vault_hkdf_print()
 *
 * @brief   Central logging function for HKDF tests
 *
 * @param   level       The log level for the specified message
 *
 * @param   test_case   The test case number associated with the message
 *
 * @param   p_str       The message to print
 *
 ********************************************************************************************************
 */

void test_vault_hkdf_print(OCKAM_LOG_e level, uint32_t test_case, char *p_str)
{
    test_vault_print( level,
                     "HKDF",
                      test_case,
                      p_str);
}
