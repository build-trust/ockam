/**
 ********************************************************************************************************
 * @file        ockam_vault.c
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
#include <vault/inc/ockam_vault_hw.h>
#include <vault/inc/ockam_vault_crypto.h>

#include <common/inc/ockam_kal.h>

#include <config/ockam_vault_cfg.h>


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

typedef enum {
    VAULT_STATE_UNINIT                                  = 0x01, /*!< Vault is uninitialized                             */
    VAULT_STATE_IDLE                                    = 0x02  /*!< Vault is in idle                                   */
} VAULT_STATE_e;


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

static OCKAM_KAL_MUTEX g_vault_mutex;

static VAULT_STATE_e g_vault_state = VAULT_STATE_UNINIT;


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
 *                                          ockam_vault_init()
 *
 * @brief   Initialize the Ockam Vault
 *
 * @param   p_cfg   Configuration values for the hardware and/or crypto library
 * 
 * @return  OCKAM_ERR_NONE if initialized successfully. OCKAM_ERR_VAULT_ALREADY_INIT if already
 *          initialized. Other errors if specific chip fails init.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_init(OCKAM_VAULT_CFG_s *p_cfg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(g_vault_state != VAULT_STATE_UNINIT) {               /* Make sure we're not already initialized              */
            ret_val = OCKAM_ERR_VAULT_ALREADY_INIT;
            break;
        }

                                                                /* Create a mutex for vault access                      */
        ret_val = ockam_kal_mutex_init(&g_vault_mutex);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }


#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_CFG_HW)
        ret_val = ockam_vault_hw_init(p_cfg->p_hw);         /* Initialize the hw code if needed                     */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }
#endif

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_CFG_CRYPTO)
        ret_val = ockam_vault_crypto_init(p_cfg->p_crypto);     /* Initialize the crypto lib code if needed             */

        if(ret_val != OCKAM_ERR_NONE) {                         /* If the crypto lib fails, free the hw if necessary    */
#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_CFG_HW)
                ockam_vault_hw_free();
#endif
            }
            break;
        }
#endif

        g_vault_state = VAULT_STATE_IDLE;                       /* Set the vault state to idle so it can be used        */
    } while(0);

    if(ret_val != OCKAM_ERR_NONE) {                             /* If init fails, release any mutexes acquired          */
        ockam_kal_mutex_free(&g_vault_mutex);                   /*  No need to check return, free may fail if it was    */ 
    }                                                           /*  never acquired.                                     */

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                        ockam_vault_random()
 *
 * @brief   Generate and return a random number
 *
 * @param   p_rand_num[out]     32-byte array to be filled with the random number
 *
 * @param   rand_num_size[in]   The size of the desired random number & buffer passed in. Used to verify
 *                              correct size.
 * 
 * @return  OCKAM_ERR_NONE if successful. OCKAM_ERR_VAULT_INVALID_SIZE if size 
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val;
    OCKAM_ERR t_ret_val;

    do {
        if((p_rand_num == OCKAM_NULL) || (rand_num_size == 0)) {/* Ensure the buffer is not null and the random size is */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* greater than 0                                       */
            break;
        }

        ret_val = ockam_kal_mutex_lock(&g_vault_mutex, 0, 0);   /* Lock the mutex before checking the state             */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        if(g_vault_state != VAULT_STATE_IDLE) {                 /* Ensure vault is in an idle state before continuing   */
            ret_val = OCKAM_ERR_INVALID_STATE;
            break;
        }

#if(OCKAM_VAULT_CFG_RAND & OCKAM_VAULT_CFG_HW)
        ret_val = ockam_vault_hw_random(p_rand_num,             /* Get a random number from hardware                    */
                                        rand_num_size);
#elif(OCKAM_VAULT_CFG_RAND & OCKAM_VAULT_CFG_CRYPTO)
        ret_val = ockam_vault_crypto_random(p_rand_num,         /* Get a random number from the crypto lib              */
                                            rand_num_size);
#else
#error "Ockam Vault: Random function not specified"
#endif
    } while(0);

    t_ret_val = ockam_kal_mutex_unlock(&g_vault_mutex, 0);      /* Unlock the mutex after all vault operations finish   */
    if(ret_val == OCKAM_ERR_NONE) {                             /* Don't overwrite ret_val if there was an error before */
        ret_val = t_ret_val;                                    /* the mutex unlock                                     */
    }

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_vault_key_gen()
 *
 * @brief   Generate an ECC keypair and get the public key
 *
 * @param   key_type[in]        The type of key pair to generate.
 *
 * @param   p_key_pub[out]      Buffer to place the public key in
 *
 * @param   key_pub_size[in]    The size of the key buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_key_gen(OCKAM_VAULT_KEY_e key_type, uint8_t *p_key_pub, uint32_t key_pub_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OCKAM_ERR t_ret_val = OCKAM_ERR_NONE;


    do {
        if((p_key_pub == OCKAM_NULL) || (key_pub_size == 0)) {  /* Ensure the buffer is not null and the key size is    */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* greater than zero                                    */
        }

        ret_val = ockam_kal_mutex_lock(&g_vault_mutex, 0, 0);   /* Lock the mutex before checking the state or          */
        if(ret_val != OCKAM_ERR_NONE) {                         /* generating a key                                     */
            break;
        }

        if(g_vault_state != VAULT_STATE_IDLE) {                 /* Ensure vault is in an idle state before continuing   */
            ret_val = OCKAM_ERR_INVALID_STATE;
            break;
        }

#if(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_HW)
        ret_val = ockam_vault_hw_key_gen(key_type,              /* Generate a key in hardware                           */
                                         p_key_pub,
                                         key_pub_size);
#elif(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_CRYPTO)
        ret_val = ockam_vault_crypto_key_gen(key_type,          /* Generate a key using the crypto lib                  */
                                             p_key_pub,
                                             key_pub_size);
#else
#error "Ockam Vault: Key Gen Function Missing"
#endif
    } while(0);

    t_ret_val = ockam_kal_mutex_unlock(&g_vault_mutex, 0);      /* Unlock the mutex after all vault operations finish   */
    if(ret_val == OCKAM_ERR_NONE) {                             /* Don't overwrite ret_val if there was an error before */
        ret_val = t_ret_val;                                    /* the mutex unlock                                     */
    }

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_vault_key_get_pub()
 *
 * @brief   Get a public key from the ATECC508A
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

OCKAM_ERR ockam_vault_key_get_pub(OCKAM_VAULT_KEY_e key_type, uint8_t *p_key_pub, uint32_t key_pub_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OCKAM_ERR t_ret_val = OCKAM_ERR_NONE;


    do {
        if((p_key_pub == OCKAM_NULL) || (key_pub_size == 0)) {  /* Ensure the buffer is not null and the key size is    */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* greater than zero                                    */
        }

        ret_val = ockam_kal_mutex_lock(&g_vault_mutex, 0, 0);   /* Lock the mutex before checking the state or          */
        if(ret_val != OCKAM_ERR_NONE) {                         /* getting the public key                               */
            break;
        }

        if(g_vault_state != VAULT_STATE_IDLE) {                 /* Ensure vault is in an idle state before continuing   */
            ret_val = OCKAM_ERR_INVALID_STATE;
            break;
        }

#if(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_HW)
        ret_val = ockam_vault_hw_key_get_pub(key_type,          /* Get a public key from hardware                       */
                                             p_key_pub,
                                             key_pub_size);
#elif(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_CRYPTO)
        ret_val = ockam_vault_crypto_key_get_pub(key_type,      /* Get a public key from the crypto lib                 */
                                                 p_key_pub,
                                                 key_pub_size);
#else
#error "Ockam Vault: Key Get Pub Function Missing"
#endif
    } while(0);

    t_ret_val = ockam_kal_mutex_unlock(&g_vault_mutex, 0);      /* Unlock the mutex after all vault operations finish   */
    if(ret_val == OCKAM_ERR_NONE) {                             /* Don't overwrite ret_val if there was an error before */
        ret_val = t_ret_val;                                    /* the mutex unlock                                     */
    }

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_vault_ecdh()
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

OCKAM_ERR ockam_vault_ecdh(OCKAM_VAULT_KEY_e key_type,
                           uint8_t *p_key_pub,
                           uint32_t key_pub_size,
                           uint8_t *p_pms,
                           uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OCKAM_ERR t_ret_val = OCKAM_ERR_NONE;


    do {
        if((p_key_pub == OCKAM_NULL) || (key_pub_size == 0) ||  /* Ensure the buffers are not null and the size values  */
            p_pms == OCKAM_NULL || pms_size == 0) {             /* are greater than zero                                */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

        ret_val = ockam_kal_mutex_lock(&g_vault_mutex, 0, 0);   /* Lock the mutex before checking the state or          */
        if(ret_val != OCKAM_ERR_NONE) {                         /* performing the ECDH operation                        */
            break;
        }

        if(g_vault_state != VAULT_STATE_IDLE) {                 /* Ensure vault is in an idle state before continuing   */
            ret_val = OCKAM_ERR_INVALID_STATE;
            break;
        }

#if(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_HW)
        ret_val = ockam_vault_hw_ecdh(key_type,                 /* Perform an ECDH operation in hardware                */
                                      p_key_pub,
                                      key_pub_size,
                                      p_pms,
                                      pms_size);
#elif(OCKAM_VAULT_CFG_KEY_ECDH & OCKAM_VAULT_CFG_CRYPTO)
        ret_val = ockam_vault_crypto_ecdh(key_type,             /* Perform an ECDH operation in the crypto library      */
                                          p_key_pub,
                                          key_pub_size,
                                          p_pms,
                                          pms_size);
#else
#error "Ockam Vault: ECDH Function missing"
#endif
    } while(0);

    t_ret_val = ockam_kal_mutex_unlock(&g_vault_mutex, 0);      /* Unlock the mutex after all vault operations finish   */
    if(ret_val == OCKAM_ERR_NONE) {                             /* Don't overwrite ret_val if there was an error before */
        ret_val = t_ret_val;                                    /* the mutex unlock                                     */
    }

    return ret_val;
}


