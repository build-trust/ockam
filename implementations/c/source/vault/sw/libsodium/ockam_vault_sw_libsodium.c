/**
 ********************************************************************************************************
 * @file        ockam_vault_sw_libsodium.c
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

#include <vault/inc/ockam_vault.h>
#include <vault/inc/ockam_vault_sw.h>

#include <common/inc/ockam_def.h>
#include <common/inc/ockam_err.h>
#include <common/inc/ockam_kal.h>


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
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */


/**
 ********************************************************************************************************
 *                                         ockam_vault_sw_init()
 *
 * @brief   Initialize libsodium for crypto operations
 *
 * @param   p_arg   Optional void* argument
 * 
 * @return  OCKAM_ERR_NONE if initialized successfully. OCKAM_ERR_VAULT_ALREADY_INIT if already
 *          initialized. Other errors if library fails.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_sw_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
    } while(0);

    return ret_val;
}


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

OCKAM_ERR ockam_vault_sw_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
    } while (0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_key_gen()
 *
 * @brief   Generate an keypair using libsodium
 *
 * @param   key_type[in]    The type of key pair to generate.
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_sw_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Static private key                                   */
        }

        else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {        /* Generate a temp key                                  */
        }

        else {                                                  /* Invalid parameter, return an error                   */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }
    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_key_get_pub()
 *
 * @brief   Get a public key the generated key
 *
 * @param   key_type[in]        OCKAM_VAULT_KEY_STATIC if requesting static public key
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

OCKAM_ERR ockam_vault_sw_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                     uint8_t *p_pub_key,
                                     uint32_t pub_key_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
        if(p_pub_key == OCKAM_NULL) {                           /* Ensure the buffer isn't null */
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        // TODO check keysize?

        switch(key_type) {
            case OCKAM_VAULT_KEY_STATIC:                        /* Get the static public key                            */
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Get the generated ephemeral public key               */
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}



/**
 ********************************************************************************************************
 *                                        ockam_vault_sw_ecdh()
 *
 * @brief   Perform ECDH using the specified key
 *
 * @param   key_type[in]        Specify which key type to use in the ECDH execution
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

OCKAM_ERR ockam_vault_sw_ecdh(OCKAM_VAULT_KEY_e key_type,
                              uint8_t *p_pub_key,
                              uint32_t pub_key_size,
                              uint8_t *p_pms,
                              uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;


    do {
        if((p_pub_key == OCKAM_NULL) ||                         /* Ensure the buffers are not null                      */
           (p_pms == OCKAM_NULL))
        {
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        //TODO validate key sizes

        switch(key_type) {

            case OCKAM_VAULT_KEY_STATIC:                        /* If using the static key, specify which slot          */
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Ephemeral key uses the temp key slot on the ATECC508A*/
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}

