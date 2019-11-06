/**
 ********************************************************************************************************
 * @file        ockam_vault_sw_mbedtls.c
 * @author      Mark Mulrooney <mark@ockam.io>
 * @copyright   Copyright (c) 2019, Ockam Inc.
 * @brief   
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam_def.h>
#include <ockam_err.h>

#include <kal/ockam_kal.h>
#include <vault/ockam_vault.h>
#include <vault/ockam_vault_sw.h>

#include "mbedtls/entropy.h"
#include "mbedtls/ctr_drbg.h"
#include "mbedtls/md.h"
#include "mbedtls/hkdf.h"

#include <ockam_vault_cfg.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define VAULT_SW_MBEDTLS_PUB_KEY_SIZE               (32u)
#define VAULT_SW_MBEDTLS_PRIV_KEY_SIZE              (32u)


/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @enum    VAULT_SW_MBEDTLS_KEY_e
 * @brief   Type of keys stored in the key struct
 *******************************************************************************
 */
typedef enum {
    VAULT_SW_MBEDTLS_KEY_PUB = 0,                               /*!< Public key identifier                              */
    VAULT_SW_MBEDTLS_KEY_PRIV,                                  /*!< Private key identifier                             */
    MAX_VAULT_SW_MBEDTLS_KEY                                    /*!< Total number of key identifiers                    */
} VAULT_SW_MBEDTLS_KEY_e;


/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */


/**
 *******************************************************************************
 * @struct  VAULT_SW_MBEDTLS_KEY_s
 * @brief
 *******************************************************************************
 */
typedef struct {
    uint8_t pub_data[VAULT_SW_MBEDTLS_PUB_KEY_SIZE];            /*!< Public key data                                    */
    uint8_t priv_data[VAULT_SW_MBEDTLS_PRIV_KEY_SIZE];          /*!< Private key data                                   */
    uint8_t valid;                                              /*!< OCKAM_FALSE if invalid, OCKAM_TRUE if valid        */
} VAULT_SW_MBEDTLS_KEY_s;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

OCKAM_ERR vault_sw_mbedtls_get_key(OCKAM_VAULT_KEY_e key,
                                   VAULT_SW_MBEDTLS_KEY_e key_type,
                                   uint8_t *p_key);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH  == OCKAM_VAULT_SW_MBEDTLS)
VAULT_SW_MBEDTLS_KEY_s* g_key[MAX_OCKAM_VAULT_KEY];              /* Array of buffers for key storage                    */
#endif

#if(OCKAM_VAULT_CFG_RAND == OCKAM_VAULT_SW_MBEDTLS)
mbedtls_entropy_context g_entropy_ctx;
#endif


/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */


/**
 ********************************************************************************************************
 *                                         ockam_vault_sw_init()
 *
 * @brief   Initialize mbedtls for crypto operations
 *
 * @param   p_arg   Optional void* argument
 * 
 * @return  OCKAM_ERR_NONE if initialized successfully. 
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
#if(OCKAM_VAULT_CFG_KEY_ECDH  == OCKAM_VAULT_SW_MBEDTLS)
        for(i = 0; i < MAX_OCKAM_VAULT_KEY; i++) {
            ret_val = ockam_mem_alloc(sizeof(VAULT_SW_MBEDTLS_KEY_s), &g_key[i]);
            if(ret_val != OCKAM_ERR_NONE) {
                break;
            }
        }

        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }
#endif

#if(OCKAM_VAULT_CFG_RAND  == OCKAM_VAULT_SW_MBEDTLS)

        mbedtls_entropy_init(&g_entropy_ctx);                   /* Initialize entropy context before adding a source    */
        mbedtls_ctr_drbg_init(&g_ctr_drbg);                     /* Initialize entropy context before adding a source    */

        
        rtn = 



#endif

    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_random()
 *
 * @brief   Generate and return a random number
 *
 * @param   p_rand_num[out]     32-byte array to be filled with the random number.
 *
 * @param   rand_num_size[in]   The size of the desired random number & buffer passed in. Used to verify
 *                              correct size.
 * 
 * @return  OCKAM_ERR_NONE if successful. OCKAM_ERR_VAULT_INVALID_SIZE if size.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_RAND == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
    } while (0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_key_gen()
 *
 * @brief   Generate an keypair using mbedtls
 *
 * @param   vault_key[in]   The type of key pair to generate.
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_key_gen(OCKAM_VAULT_KEY_e vault_key)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_key_get_pub()
 *
 * @brief   Get a public key the generated key
 *
 * @param   vault_key[in]       OCKAM_VAULT_KEY_STATIC if requesting static public key
 *                              OCKAM_VAULT_KEY_EPHEMERAL if requesting the ephemeral public key
 *
 * @param   p_pub_key[out]      Buffer to place the public key in
 *
 * @param   pub_key_size[in]    Size of the public key buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_key_get_pub(OCKAM_VAULT_KEY_e vault_key,
                                     uint8_t *p_pub_key,
                                     uint32_t pub_key_size)
{
    uint8_t *p_key;
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    VAULT_SW_MBEDTLS_KEY_s *p_key;

    do
    {
        if((p_pub_key == OCKAM_NULL) ||                         /* Ensure the buffer isn't null and the size is correct */
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        if(pub_key_size != VAULT_SW_MBEDTLS_KEY_SIZE) {
            ret_val = OCKAM_ERR_SIZE_MISMATCH;
            break;
        }

        ret_val = vault_sw_mbedtls_get_key(vault_key,           /* Get the desired public key                           */
                                           VAULT_SW_MBEDTLS_KEY_PUB,
                                           p_key);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        for(i = 0; i < MAX_OCKAM_VAULT_KEY; i++) {              /* Loop through the vault keys to get the right key     */
            if(vault_key == i) {
                p_key = g_key[i];
                valid_key = OCKAM_TRUE;
                break;                                          /* Break out of for loop when key is found              */
            }
        }

        if(valid_key == OCKAM_FALSE) {                          /* Make sure we found a valid vault key, otherwise      */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* an error                                             */
            break;
        }

        if(p_key->valid == OCKAM_FALSE) {                       /* Ensure the data in the key is valid data             */
            ret_val = OCKAM_ERR_SW_KEY_FAIL;
            break;
        }

        ret_val = ockam_mem_copy(p_pub_key,                     /* Copy the public key to the buffer                    */
                                 p_key->data,
                                 pub_key_size);
    } while (0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_ecdh()
 *
 * @brief   Perform ECDH using the specified key
 *
 * @param   vault_key[in]       Specify which vault key to use in the ECDH execution
 *
 * @param   p_pub_key[in]       Buffer with the public key
 *
 * @param   pub_key_size[in]    Size of the public key buffer
 *
 * @param   p_pms[out]          Pre-master secret from ECDH
 *
 * @param   pms_size[in]        Size of the pre-master secret buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_ecdh(OCKAM_VAULT_KEY_e vault_key,
                              uint8_t *p_pub_key,
                              uint32_t pub_key_size,
                              uint8_t *p_pms,
                              uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;


    do {
        if((p_pub_key == OCKAM_NULL) || (p_pms == OCKAM_NULL))  /* Ensure the buffers are not null                      */
        {
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        if(valid_key == OCKAM_FALSE) {                          /* Ensure the vault key is valid                         */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }
    } while (0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                          ockam_vault_sw_hkdf()
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

#if(OCKAM_VAULT_CFG_HKDF == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR ockam_vault_sw_hkdf(uint8_t *p_salt,
                              uint32_t salt_size,
                              uint8_t *p_ikm,
                              uint32_t ikm_size,
                              uint8_t *p_info,
                              uint32_t info_size,
                              uint8_t *p_out,
                              uint32_t out_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    const mbedtls_md_info_t *p_md;
    int32_t ret;


    do {
        if((p_ikm == OCKAM_NULL) || (ikm_size == 0) ||          /* Ensure the input key and output buffers are not null */
            p_out == OCKAM_NULL || out_size  == 0) {            /* and the size values are greater than zero            */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

        p_md = mbedtls_md_info_from_type(MBEDTLS_MD_SHA256);    /* Get the SHA-256 MD context for HKDF                  */

        ret = mbedtls_hkdf(p_md,                                /* Perform the HKDF calculation                         */
                           p_salt, salt_size,
                           p_ikm, ikm_size,
                           p_info, info_size,
                           p_out, out_size);
        if(ret != 0) {                                          /* Check for an mbed TLS error                          */
            OCKAM_ERR_VAULT_SW_HKDF_FAIL;
        }

    } while(0);

    return ret_val;
}
#endif


/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */


/**
 ********************************************************************************************************
 *                                      vault_sw_mbedtls_get_key()
 *
 * @brief   Get a public or private key from the mbedtls storage
 *
 * @param   key             The Ockam vault key pair to retrive
 *
 * @param   vault_key[in]   The key type to retrieve (public or private)
 *
 * @param   p_key[out]      Pointer to place the key in
 * 
 * @return  OCKAM_ERR_NONE on success.
 *          OCKAM_ERR_VAULT_SW_KEY_FAIL if key is not found or if its not valid
 * 
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH  == OCKAM_VAULT_SW_MBEDTLS)
OCKAM_ERR vault_sw_mbedtls_get_key(OCKAM_VAULT_KEY_e vault_key,
                                   VAULT_SW_MBEDTLS_KEY_e key_type,
                                   uint8_t *p_key)
{
    uint32_t i;
    uint8_t valid_key = OCKAM_FALSE;
    OCKAM_ERR ret_val = OCKAM_ERR_VAULT_SW_KEY_FAIL;


    for(i = 0; i < MAX_OCKAM_VAULT_KEY; i++) {              /* Loop through the key types to get the right key      */
        if(vault_key == i) {
            if((g_key[i]->valid) == OCKAM_FALSE) {          /* Check to see if the keypair is valid                 */
                break;
            }

            switch(key_type) {                              /* Grab the public or private data pointer              */
                case VAULT_SW_MBEDTLS_KEY_PUB:
                    p_key = g_key[i]->pub_data;
                    ret_val = OCKAM_ERR_NONE;
                    break;

                case VAULT_SW_MBEDTLS_KEY_PRIV:
                    p_key = g_key[i]->priv_data;
                    ret_val = OCKAM_ERR_NONE;
                    break;

                default:
                    break;
            }

            break;                                          /* Break out of the for loop and return                 */
        }
    }

    return ret_val;
}
#endif

