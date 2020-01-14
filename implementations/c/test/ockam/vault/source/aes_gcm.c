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

#include <ockam/error.h>
#include <ockam/log.h>
#include <ockam/vault.h>

#include <test_vault.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define TEST_VAULT_AES_GCM_CASES                     1u

#define TEST_VAULT_AES_GCM_KEY_SIZE                 16u
#define TEST_VAULT_AES_GCM_TAG_SIZE                 16u


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


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

void test_vault_aes_gcm_print(OCKAM_LOG_e level, uint32_t test_case, char *p_str);


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


TEST_VAULT_AES_GCM_DATA_s g_aes_gcm_data[TEST_VAULT_AES_GCM_CASES] =
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
    }
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
 ********************************************************************************************************
 */

void test_vault_aes_gcm(void)
{
    OCKAM_ERR err = OCKAM_ERR_NONE;
    int ret = 0;
    uint8_t i = 0;


    for(i = 0; i < TEST_VAULT_AES_GCM_CASES; i++) {
        uint8_t aes_gcm_tag[TEST_VAULT_AES_GCM_TAG_SIZE];
        uint8_t aes_gcm_encrypt_hash[g_aes_gcm_data[i].text_size];
        uint8_t aes_gcm_decrypt_data[g_aes_gcm_data[i].text_size];


        /* --------------- */
        /* AES GCM Encrypt */
        /* --------------- */

        err = ockam_vault_aes_gcm_encrypt( g_aes_gcm_data[i].p_key,
                                           TEST_VAULT_AES_GCM_KEY_SIZE,
                                           g_aes_gcm_data[i].p_iv,
                                           g_aes_gcm_data[i].iv_size,
                                           g_aes_gcm_data[i].p_aad,
                                           g_aes_gcm_data[i].aad_size,
                                          &aes_gcm_tag[0],
                                           TEST_VAULT_AES_GCM_TAG_SIZE,
                                           g_aes_gcm_data[i].p_plain_text,
                                           g_aes_gcm_data[i].text_size,
                                          &aes_gcm_encrypt_hash[0],
                                           g_aes_gcm_data[i].text_size);
        if(err != OCKAM_ERR_NONE) {
            test_vault_aes_gcm_print(OCKAM_LOG_ERROR,
                                     i,
                                     "Encrypt Operation Failed");
        }

        ret = memcmp(&aes_gcm_tag[0],                           /* Compare the computed tag with the expected tag     */
                      g_aes_gcm_data[i].p_tag,
                      TEST_VAULT_AES_GCM_TAG_SIZE);
        if(ret != 0) {
            test_vault_aes_gcm_print(OCKAM_LOG_ERROR,
                                     i,
                                    "Calculated Encrypt Tag Invalid");
            test_vault_print_array( OCKAM_LOG_DEBUG,
                                   "AES GCM",
                                   "Tag : Calculated Value",
                                   &aes_gcm_tag[0],
                                    TEST_VAULT_AES_GCM_TAG_SIZE);

            test_vault_print_array( OCKAM_LOG_DEBUG,
                                   "AES GCM",
                                   "Tag : Expected Value",
                                    g_aes_gcm_data[i].p_tag,
                                    TEST_VAULT_AES_GCM_TAG_SIZE);
        } else {
            test_vault_aes_gcm_print(OCKAM_LOG_INFO,
                                     i,
                                     "Calculated Encrypt Tag Valid");
        }


        ret = memcmp(&aes_gcm_encrypt_hash[0],                  /* Compare the computed hash with the expected hash   */
                      g_aes_gcm_data[i].p_encrypted_text,
                      g_aes_gcm_data[i].text_size);
        if(ret != 0) {
            test_vault_aes_gcm_print(OCKAM_LOG_ERROR,
                                     i,
                                     "Calculated Encrypt Hash Invalid");

        } else {
            test_vault_aes_gcm_print(OCKAM_LOG_INFO,
                                     i,
                                     "Calculated Encrypt Hash Valid");
        }

        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "AES GCM",
                               "Encrypted Hash : Calculated Value",
                               &aes_gcm_encrypt_hash[0],
                                g_aes_gcm_data[i].text_size);

        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "AES GCM",
                               "Encrypted Hash : Expected Value",
                                g_aes_gcm_data[i].p_encrypted_text,
                                g_aes_gcm_data[i].text_size);

        /* --------------- */
        /* AES GCM Decrypt */
        /* --------------- */

        err = ockam_vault_aes_gcm_decrypt( g_aes_gcm_data[i].p_key,
                                           TEST_VAULT_AES_GCM_KEY_SIZE,
                                           g_aes_gcm_data[i].p_iv,
                                           g_aes_gcm_data[i].iv_size,
                                           g_aes_gcm_data[i].p_aad,
                                           g_aes_gcm_data[i].aad_size,
                                           g_aes_gcm_data[i].p_tag,
                                           TEST_VAULT_AES_GCM_TAG_SIZE,
                                           g_aes_gcm_data[i].p_encrypted_text,
                                           g_aes_gcm_data[i].text_size,
                                          &aes_gcm_decrypt_data[0],
                                           g_aes_gcm_data[i].text_size);
        if(err != OCKAM_ERR_NONE) {
            test_vault_aes_gcm_print(OCKAM_LOG_ERROR,
                                     i,
                                     "Decrypt Operation Failed");
        }

        ret = memcmp(&aes_gcm_decrypt_data[0],                  /* Compare the computed hash with the expected hash   */
                      g_aes_gcm_data[i].p_plain_text,
                      g_aes_gcm_data[i].text_size);
        if(ret != 0) {
            test_vault_aes_gcm_print(OCKAM_LOG_ERROR,
                                     i,
                                     "Calculated Decrypted Hash Invalid");
        } else {
            test_vault_aes_gcm_print(OCKAM_LOG_INFO,
                                     i,
                                     "Calculated Decrypted Hash Valid");
        }

        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "AES GCM",
                               "Decrypted Hash : Calculated Value",
                               &aes_gcm_decrypt_data[0],
                               g_aes_gcm_data[i].text_size);
        test_vault_print_array(OCKAM_LOG_DEBUG,
                               "AES GCM",
                               "Decrypted Hash : Expected Value",
                                g_aes_gcm_data[i].p_plain_text,
                                g_aes_gcm_data[i].text_size);

    }
}


/**
 ********************************************************************************************************
 *                                          test_vault_aes_gcm_print()
 *
 * @brief   AES GCM print function
 *
 * @param   level       The level at which to log the message at
 *
 * @param   test_case   The test case number associated with the message
 *
 * @param   p_str       Null-terminated string message to print
 *
 ********************************************************************************************************
 */

void test_vault_aes_gcm_print(OCKAM_LOG_e level, uint32_t test_case, char *p_str)
{
    test_vault_print( level,
                     "AES GCM",
                      test_case,
                      p_str);
}

