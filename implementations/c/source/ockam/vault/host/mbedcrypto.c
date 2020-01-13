/**
 ********************************************************************************************************
 * @file    mbedcrypto.c
 * @brief   mbedcrypto implementation of Ockam Vault functions
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam/define.h>
#include <ockam/error.h>
#include <ockam/kal.h>
#include <ockam/vault.h>
#include <ockam/vault/host.h>

#include "mbedtls/entropy.h"
#include "mbedtls/ctr_drbg.h"
#include "mbedtls/md.h"
#include "mbedtls/hkdf.h"
#include "mbedtls/gcm.h"

#if !defined(OCKAM_VAULT_CONFIG_FILE)
#error "Error: Ockam Vault Config File Missing"
#else
#include OCKAM_VAULT_CONFIG_FILE
#endif


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */


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


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */


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
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_INIT
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HOST_MBEDCRYPTO)


/**
 ********************************************************************************************************
 *                                         ockam_vault_host_init()
 *
 * @brief   Initialize mbedtls for crypto operations
 *
 * @param   p_arg   Optional void* argument
 *
 * @return  OCKAM_ERR_NONE if initialized successfully.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;

    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_INIT                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_RAND
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_RAND == OCKAM_VAULT_HOST_MBEDCRYPTO)
#error "Error: OCKAM_VAULT_CFG_RAND invalid for MBEDCRYPTO"
#endif                                                          /* OCKAM_VAULT_CFG_RAND                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                      OCKAM_VAULT_CFG_KEY_ECDH
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_HOST_MBEDCRYPTO)
#error "Error: OCKAM_VAULT_CFG_KEY_ECHD invalid for MBEDCRYPTO"
#endif                                                          /* OCKAM_VAULT_CFG_KEY_ECDH                           */


/**
 ********************************************************************************************************
 ********************************************************************************************************
 *                                       OCKAM_VAULT_CFG_HKDF
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_HKDF == OCKAM_VAULT_HOST_MBEDCRYPTO)


/**
 ********************************************************************************************************
 *                                          ockam_vault_host_hkdf()
 *
 * @brief   Perform HKDF in the mbed TLS library
 *
 * @param   p_salt[in]          Buffer for the Ockam salt value
 *
 * @param   salt_size[in]       Size of the Ockam salt value
 *
 * @param   p_ikm[in]           Buffer with the input key material for HKDF
 *
 * @param   ikm_size[in]        Size of the input key material
 *
 * @param   p_info[in]          Buffer with the optional context specific info. Can be 0.
 *
 * @param   info_size[in]       Size of the optional context specific info.
 *
 * @param   p_out[out]          Buffer for the output of the HKDF operation
 *
 * @param   out_size[in]        Size of the HKDF output buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_hkdf(uint8_t *p_salt, uint32_t salt_size,
                                uint8_t *p_ikm, uint32_t ikm_size,
                                uint8_t *p_info, uint32_t info_size,
                                uint8_t *p_out, uint32_t out_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    const mbedtls_md_info_t *p_md;
    int32_t mbed_ret;


    do {
        if((p_ikm == 0) || (ikm_size == 0) ||                   /* Ensure the input key and output buffers are not    */
           (p_out == 0) || (out_size  == 0)) {                  /* null and the size values are greater than zero     */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

        p_md = mbedtls_md_info_from_type(MBEDTLS_MD_SHA256);    /* Get the SHA-256 MD context for HKDF                */

        mbed_ret = mbedtls_hkdf(p_md,                           /* Perform the HKDF calculation                       */
                           p_salt, salt_size,
                           p_ikm, ikm_size,
                           p_info, info_size,
                           p_out, out_size);
        if(mbed_ret != 0) {                                     /* Check for an mbed TLS error                        */
            OCKAM_ERR_VAULT_HOST_HKDF_FAIL;
        }

    } while(0);

    return ret_val;
}


#endif                                                          /* OCKAM_CFG_VAULT_HKDF                               */



/**
 ********************************************************************************************************
 ********************************************************************************************************
 *                                      OCKAM_VAULT_CFG_AES_GCM
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_AES_GCM == OCKAM_VAULT_HOST_MBEDCRYPTO)


/**
 ********************************************************************************************************
 *                                       ockam_vault_host_aes_gcm()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_aes_gcm(OCKAM_VAULT_AES_GCM_MODE_e mode,
                                   uint8_t *p_key, uint32_t key_size,
                                   uint8_t *p_iv, uint32_t iv_size,
                                   uint8_t *p_aad, uint32_t aad_size,
                                   uint8_t *p_tag, uint32_t tag_size,
                                   uint8_t *p_input, uint32_t input_size,
                                   uint8_t *p_output, uint32_t output_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int32_t mbed_ret;
    uint32_t key_bit_size = 0;
    mbedtls_gcm_context gcm;


    do {
        if((p_key == 0) || (key_size == 0) ||                   /* Ensure there are no null buffers or sizes set to   */
           (p_iv == 0) || (iv_size == 0) ||                     /* 0, every pointer needs to be valid and sizes must  */
           (p_tag == 0) || (tag_size == 0) ||                   /* always be greater than 0.                          */
           (p_input == 0) || (input_size == 0) ||
           (p_output == 0) || (output_size == 0)) {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        key_bit_size = key_size * 8;                            /* Key size is specified in bits. Ensure the key      */
        if((key_bit_size != 128) &&                             /* size is either 128, 192 or 256 bytes.              */
           (key_bit_size != 192) &&
           (key_bit_size != 256)) {
            ret_val = OCKAM_ERR_VAULT_INVALID_KEY_SIZE;
            break;
        }

        if(p_input == p_output) {
            ret_val = OCKAM_ERR_VAULT_INVALID_BUFFER;           /* The input buffer can not be used for the result    */
            break;
        }

        if(input_size != output_size) {                         /* Input buffer size must match the output buffer     */
            ret_val = OCKAM_ERR_VAULT_INVALID_BUFFER_SIZE;      /* size, otherwise encrypt/decyrpt fails              */
            break;
        }

        do {
            mbedtls_gcm_init(&gcm);                             /* Always initialize the AES GCM context first        */

            mbed_ret = mbedtls_gcm_setkey(&gcm,                 /* Set the AES key. Key size must be specified in     */
                                          MBEDTLS_CIPHER_ID_AES,/* bits.                                              */
                                          p_key,
                                          key_bit_size);
            if(mbed_ret != 0) {                                 /* TODO allow platform feature unsupported?           */
                ret_val = OCKAM_ERR_VAULT_HOST_AES_FAIL;
                break;
            }

            if(mode == OCKAM_VAULT_AES_GCM_MODE_ENCRYPT) {      /* For encrypt, encrypt the supplied data, IV, and    */
                mbed_ret = mbedtls_gcm_crypt_and_tag(&gcm,      /* optional aad data to get encrypted output and tag  */
                                                     MBEDTLS_GCM_ENCRYPT,
                                                     input_size,
                                                     p_iv,
                                                     iv_size,
                                                     p_aad,
                                                     aad_size,
                                                     p_input,
                                                     p_output,
                                                     tag_size,
                                                     p_tag);
                if(mbed_ret != 0) {
                    ret_val = OCKAM_ERR_VAULT_HOST_AES_FAIL;
                    break;
                }
            } else if(mode == OCKAM_VAULT_AES_GCM_MODE_DECRYPT) {
                mbed_ret = mbedtls_gcm_auth_decrypt(&gcm,       /* For decrypt, supply the input data, IV, optional   */
                                                    input_size, /* aad data, and the tag to get the decrypted output  */
                                                    p_iv,
                                                    iv_size,
                                                    p_aad,
                                                    aad_size,
                                                    p_tag,
                                                    tag_size,
                                                    p_input,
                                                    p_output);
                if(mbed_ret != 0) {
                    ret_val = OCKAM_ERR_VAULT_HOST_AES_FAIL;
                    break;
                }
            } else {                                            /* Any modes besides encrypt and decrypt are invalid  */
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
            }
        } while(0);

        mbedtls_gcm_free(&gcm);                                 /* Always attempt to free even if an error occurred   */
    } while(0);

    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_AES_GCM                            */

