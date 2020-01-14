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
#include <ockam/memory.h>
#include <ockam/vault.h>
#include <ockam/vault/host.h>

#include "mbedtls/ecp.h"
#include "mbedtls/entropy.h"
#include "mbedtls/ctr_drbg.h"
#include "mbedtls/md.h"
#include "mbedtls/hkdf.h"
#include "mbedtls/gcm.h"
#include "mbedtls/sha256.h"

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

#define MBEDCRYPTO_KEY_CURVE25519_STATIC            0u
#define MBEDCRYPTO_KEY_CURVE25519_EPHEMERAL         1u
#define MBEDCRYPTO_KEY_CURVE25519_TOTAL             2u

#define MBEDCRYPTO_SHA256_IS224                     0u          /* Used to specify SHA256 rather than SHA224          */


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

uint32_t g_mbedcrypto_str_len = 23;
char *g_mbedcrypto_str = "ockam_mbedcrypto_string";

mbedtls_entropy_context g_entropy;
mbedtls_ctr_drbg_context g_ctr_drbg;
mbedtls_ecp_keypair g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_TOTAL];


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
    int mbed_ret = 0;


    do {
        mbedtls_entropy_init(&g_entropy);                       /* Initialize the entropy before CTR DRBG. Both inits */
        mbedtls_ctr_drbg_init(&g_ctr_drbg);                     /* have no return value.                              */

        mbed_ret = mbedtls_ctr_drbg_seed(&g_ctr_drbg,           /* Seed the CTR DRBG using exisitng entropy and the   */
                                         mbedtls_entropy_func,  /* personalization string                             */
                                         &g_entropy,
                                         (const unsigned char*) g_mbedcrypto_str,
                                         g_mbedcrypto_str_len);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_INIT_FAIL;
            break;
        }
    } while(0);

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

/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_random()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int mbed_ret = 0;

    mbed_ret = mbedtls_ctr_drbg_random(&g_ctr_drbg,
                                       p_rand_num,
                                       rand_num_size);
    if(mbed_ret != 0) {
        ret_val = OCKAM_ERR_VAULT_HOST_RAND_FAIL;
    }


    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_RAND                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                      OCKAM_VAULT_CFG_KEY_ECDH
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_HOST_MBEDCRYPTO)

/*
 ********************************************************************************************************
 *                                     ockam_vault_host_key_gen()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int mbed_ret = 0;
    mbedtls_ecp_keypair *p_key = 0;


    do {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Set the keypair data based on the desired key      */
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_STATIC];
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_EPHEMERAL];
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        mbedtls_ecp_keypair_init(p_key);                        /* Always initialize the keypair first                */

                                                                /* Generate the keypair on Curve25519                 */
        mbed_ret = mbedtls_ecp_gen_key(MBEDTLS_ECP_DP_CURVE25519,
                                       p_key,
                                       mbedtls_ctr_drbg_random,
                                       &g_ctr_drbg);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_KEY_FAIL;
            break;
        }

        mbed_ret = mbedtls_ecp_check_pubkey(&(p_key->grp),      /* Check that generated public key lies on the curve  */
                                            &(p_key->Q));
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_KEY_FAIL;
            break;
        }

        mbed_ret = mbedtls_ecp_check_privkey(&(p_key->grp),     /* Check that the generated private key is valid      */
                                             &(p_key->d));
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_KEY_FAIL;
            break;
        }
    } while(0);

    return ret_val;
}


/*
 ********************************************************************************************************
 *                                      ockam_vault_host_key_get_pub()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                       uint8_t *p_pub_key,
                                       uint32_t pub_key_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int mbed_ret = 0;
    mbedtls_ecp_keypair *p_key = 0;
    size_t olen = 0;


    do {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Set the keypair data based on the desired key      */
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_STATIC];
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_EPHEMERAL];
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        mbed_ret = mbedtls_ecp_point_write_binary(&(p_key->grp),
                                                  &(p_key->Q),
                                                  MBEDTLS_ECP_PF_UNCOMPRESSED,
                                                  &olen,
                                                  p_pub_key,
                                                  pub_key_size);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_KEY_FAIL;
            break;
        }
    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                      ockam_vault_host_ecdh()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_ecdh(OCKAM_VAULT_KEY_e key_type,
                                uint8_t *p_pub_key, uint32_t pub_key_size,
                                uint8_t *p_pms, uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int mbed_ret = 0;

    mbedtls_ecp_keypair *p_key = 0;
    mbedtls_mpi pms;
    mbedtls_ecp_point pub_key;


    do {
        mbedtls_mpi_init(&pms);
        mbedtls_ecp_point_init(&pub_key);


        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Set the keypair data based on the desired key      */
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_STATIC];
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {
            p_key = &g_keypair_data[MBEDCRYPTO_KEY_CURVE25519_EPHEMERAL];
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        mbed_ret = mbedtls_ecp_point_read_binary(&(p_key->grp), /* Write the received public key to the ECDH context  */
                                                 &pub_key,
                                                 p_pub_key,
                                                 pub_key_size);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_ECDH_FAIL;
            break;
        }

        mbed_ret = mbedtls_ecdh_compute_shared(&(p_key->grp),   /* Generate the shared secret                         */
                                               &pms,
                                               &pub_key,
                                               &(p_key->d),
                                               mbedtls_ctr_drbg_random,
                                               &g_ctr_drbg);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_ECDH_FAIL;
            break;
        }

        mbed_ret = mbedtls_mpi_write_binary(&pms,               /* Write the generated PMS to the PMS buffer          */
                                            p_pms,
                                            pms_size);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_ECDH_FAIL;
            break;
        }

    } while(0);



    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_KEY_ECDH                           */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                        OCKAM_VAULT_CFG_SHA256
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_SHA256 == OCKAM_VAULT_HOST_MBEDCRYPTO)


/**
 ********************************************************************************************************
 *                                    ockam_vault_host_sha256()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_sha256(uint8_t *p_msg, uint16_t msg_size,
                                  uint8_t *p_digest, uint8_t digest_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    int mbed_ret = 0;
    mbedtls_sha256_context sha256_ctx;


    do {
        mbedtls_sha256_init(&sha256_ctx);                       /* SHA256 context structure must be inited before use */


        mbed_ret = mbedtls_sha256_starts_ret(&sha256_ctx,       /* Configure for SHA256 rather than SHA224            */
                                             MBEDCRYPTO_SHA256_IS224);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_SHA256_FAIL;
            break;
        }

        mbed_ret = mbedtls_sha256_update_ret(&sha256_ctx,       /* Add the message to the SHA256 context              */
                                             p_msg,
                                             msg_size);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_SHA256_FAIL;
            break;
        }

        mbed_ret = mbedtls_sha256_finish_ret(&sha256_ctx,       /* Complete SHA256 hash and output to digest buffer   */
                                             p_digest);
        if(mbed_ret != 0) {
            ret_val = OCKAM_ERR_VAULT_HOST_SHA256_FAIL;
            break;
        }
    } while(0);

    mbedtls_sha256_free(&sha256_ctx);                           /* Always clear the SHA256 context when finished      */

    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_SHA256                             */


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

