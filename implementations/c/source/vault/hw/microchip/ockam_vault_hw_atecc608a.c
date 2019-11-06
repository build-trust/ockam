/**
 ********************************************************************************************************
 * @file        ockam_vault_hw_atecc608a.c
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

#include <cryptoauthlib/lib/cryptoauthlib.h>
#include <cryptoauthlib/lib/atca_cfgs.h>
#include <cryptoauthlib/lib/atca_iface.h>
#include <cryptoauthlib/lib/atca_device.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define VAULT_ATECC608A_PMS_SIZE                    32u         /* Size of the pre-master secret                        */
#define VAULT_ATECC608A_RAND_SIZE                   32u         /* Size of the random number generated                  */
#define VAULT_ATECC608A_PUB_KEY_SIZE                64u         /* Size of public key                                   */

#define VAULT_ATECC608A_KEY_SLOT_STATIC              0u         /* Slot with the preloaded private key                  */
#define VAULT_ATECC608A_KEY_SLOT_EPHEMERAL   ATCA_TEMPKEY_KEYID /* Slot with the generated ephemeral key                */


/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @enum    VAULT_ATECC608A_STATE_e
 * @brief   
 *******************************************************************************
 */
typedef enum {
    VAULT_ATECC608A_STATE_UNINIT                        = 0x01, /*!< Chip is uninitialized  */
    VAULT_ATECC608A_STATE_IDLE                          = 0x02  /*!< Chip is in idle        */
} VAULT_ATECC608A_STATE_e;



/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            INLINE FUNCTIONS                                          *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

static OCKAM_VAULT_KAL_MUTEX atecc608a_mutex;

static VAULT_ATECC608A_STATE_e atecc608a_state = VAULT_ATECC608A_STATE_UNINIT;

ATCAIfaceCfg cfg_ateccx08a_i2c_coral = {                        /*!< Configuration for ATECC608A on Coral RPi board         */
    .iface_type                 = ATCA_I2C_IFACE,
    .devtype                    = ATECC608A,
    {
        .atcai2c.slave_address  = 0x60,
        .atcai2c.bus            = 1,
        .atcai2c.baud           = 100000,
    },
    .wake_delay                 = 1500,
    .rx_retries                 = 20
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
 *                                          ockam_vault_init()
 *
 * @brief   Initialize the ATECC608A for Ockam Vault
 *
 * @param   p_arg   Optional void* argument
 * 
 * @return  OCKAM_ERR_NONE if initialized successfully. OCKAM_ERR_VAULT_ALREADY_INIT if already
 *          initialized. Other errors if specific chip fails init.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
OCKAM_ERR ockam_vault_init(void *p_arg)
{
    ATCA_STATUS status;
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(atecc608a_state != VAULT_ATECC608A_STATE_UNINIT) {   /* Make sure we're not already initialized              */
            ret_val = OCKAM_ERR_VAULT_ALREADY_INIT;
            break;
        }

        ret_val = ockam_vault_kal_mutex_init(&atecc608a_mutex); /* Create a mutex for the ATECC608A                     */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        status = atcab_init(&cfg_ateccx08a_i2c_coral);          /* Call Cryptolib to initialize the ATECC608A via I2C   */
        if(status != ATCA_SUCCESS) {
            printf("Init failed with code 0x%08X\r\n", status);
            ret_val = OCKAM_ERR_VAULT_HW_INIT_FAIL;
            break;
        }



        // TODO read config
        // Check for the following:
        // -AES enabled
        // Look at slot config?
        // IO Protection Key?
        //

        atecc608a_state = VAULT_ATECC608A_STATE_IDLE;           /* Change the state to idle                             */
    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                          ockam_vault_hw_free()
 *
 * @brief   Free the hardware and all associated data structures
 *
 * @return  OCKAM_ERR_NONE on success.
 * 
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
void ockam_vault_hw_free (void)
{
   return OCKAM_ERR_NONE; 
}
#endif

/**
 ********************************************************************************************************
 *                                          ockam_vault_random()
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

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
OCKAM_ERR ockam_vault_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(rand_num_size != VAULT_ATECC608A_RAND_SIZE) {        /* Make sure the expected size matches the buffer       */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        ockam_vault_kal_mutex_lock(&atecc608a_mutex, 0, 0);     /* Lock the mutex before checking the state             */

        if(atecc608a_state != VAULT_ATECC608A_STATE_IDLE) {     /* Make sure we're in idle before executing             */
            ockam_vault_kal_mutex_unlock(&atecc608a_mutex, 0);
            break;
        }

        atcab_random(p_rand_num);                               /* Get a random number from the ATECC608A               */

        ockam_vault_kal_mutex_unlock(&atecc608a_mutex, 0);      /* Release the mutex                                    */
    } while (0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                          ockam_vault_key_gen()
 *
 * @brief   Generate an keypair on the ATECC608A
 *
 * @param   key_type[in]    The type of key pair to generate.
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
OCKAM_ERR ockam_vault_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;

    do
    {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Static private key preloaded on ATECC608A            */
            break;
        }

        else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {        /* Generate a temp key                                  */
            atcab_genkey(ATCA_TEMPKEY_KEYID, 0);
        }

        else {                                                  /* Invalid parameter, return an error                   */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                          ockam_vault_key_get_pub()
 *
 * @brief   Get a public key from the ATECC608A
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

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
OCKAM_ERR ockam_vault_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                  uint8_t *p_pub_key,
                                  uint32_t pub_key_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;


    do
    {
        if(p_pub_key == 0) {
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        if(pub_key_size != VAULT_ATECC608A_PUB_KEY_SIZE) {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        ockam_vault_kal_mutex_lock(&atecc608a_mutex, 0, 0);     /* Lock the mutex before getting the public key         */

        switch(key_type) {
            case OCKAM_VAULT_KEY_STATIC:                        /* Get the static public key                            */
                status = atcab_genkey(VAULT_ATECC608A_KEY_SLOT_STATIC,
                                      p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_KEY_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Get the generated ephemeral public key               */
                status = atcab_genkey(VAULT_ATECC608A_KEY_SLOT_EPHEMERAL,
                                       p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_KEY_FAIL;
                }
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }

        ockam_vault_kal_mutex_unlock(&atecc608a_mutex, 0);      /* Release the mutex                                    */

    } while (0);

    return ret_val;
}
#endif



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

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC608A)
OCKAM_ERR ockam_vault_ecdh(OCKAM_VAULT_KEY_e key_type,
                           uint8_t *p_pub_key,
                           uint32_t pub_key_size,
                           uint8_t *p_pms,
                           uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;


    do {
        if((p_pub_key == 0) ||                                  /* Ensure the buffers are not null                          */
           (p_pms == 0))
        {
            ret_val = OCAM_ERR_INVALID_PARAM;
            break;
        }

        if((pub_key_size != VAULT_ATECC608A_PUB_KEY_SIZE) ||    /* Validate the size of the buffers passed in               */
           (pms_size != VAULT_ATECC608A_PMS_SIZE))
        {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        ockam_vault_kal_mutex_lock(&atecc608a_mutex, 0, 0);     /* Lock the mutex before checking the state                 */

        switch(key_type) {

            case OCKAM_VAULT_KEY_STATIC:                        /* If using the static key, specify which slot              */

                status = atcab_ecdh(VAULT_ATECC608A_KEY_SLOT_STATIC,
                                    p_pub_key,
                                    p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_ECDH_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Ephemeral key uses the temp key slot on the ATECC608A    */

                status = atcab_ecdh_tempkey(p_pub_key,
                                            p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_ECDH_FAIL;
                }
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }

        ockam_vault_kal_mutex_unlock(&atecc608a_mutex, 0);      /* Release the mutex                                        */

    } while (0);


    return ret_val;
}
#endif


