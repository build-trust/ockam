/**
 ********************************************************************************************************
 * @file        ockam_vault_atecc608a.c
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

#include <stdlib.h>
#include <stdint.h>

#include <vault/inc/ockam_err.h>
#include <vault/inc/ockam_vault.h>
#include <vault/hal/ockam_vault_hal.h>

#include <cryptoauthlib/lib/cryptoauthlib.h>
#include <cryptoauthlib/lib/atca_cfgs.h>
#include <cryptoauthlib/lib/atca_iface.h>
#include <cryptoauthlib/lib/atca_device.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define VAULT_ATECC608A_RAND_SIZE                   32u


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

static OCKAM_VAULT_HAL_MUTEX atecc608a_mutex;

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
 * @return  OCKAM_ERR_NONE if initialized sucessfully. OCKAM_ERR_VAULT_ALREADY_INIT if already
 *          initialized. Other errors if specific chip fails init.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_init(void *p_arg)
{
    ATCA_STATUS status;
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(atecc608a_state != VAULT_ATECC608A_STATE_UNINIT) {   /* Make sure we're not already initialized              */
            ret_val = OCKAM_ERR_VAULT_ALREADY_INIT;
            break;
        }

        ret_val = ockam_vault_hal_mutex_init(&atecc608a_mutex); /* Create a mutex for the ATECC608A                     */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        status = atcab_init(&cfg_ateccx08a_i2c_coral);          /* Call Cryptolib to initialize the ATECC608A via I2C   */
        if(status != ATCA_SUCCESS) {
            printf("Init failed with code 0x%08X\r\n", status);
            ret_val = OCKAM_ERR_VAULT_HW_INIT_FAIL;
            break;
        }

        atecc608a_state = VAULT_ATECC608A_STATE_IDLE;           /* Change the state to idle                             */
    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_vault_random()
 *
 * @brief   Retrieve the current state of the ATECC608A
 *
 * @param   p_rand_num      32-byte array to be filled with the random number
 *
 * @param   rand_num_size   The size of the desired random number & buffer passed in. Used to verify
 *                          correct size.
 * 
 * @return  OCKAM_ERR_NONE if sucessful. OCKAM_ERR_VAULT_INVALID_SIZE if size 
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(rand_num_size != VAULT_ATECC608A_RAND_SIZE) {        /* Make sure the expected size matches the buffer   */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        ockam_vault_hal_mutex_lock(&atecc608a_mutex, 0, 0);     /* Lock the mutex before checking the state         */

        if(atecc608a_state != VAULT_ATECC608A_STATE_IDLE) {     /* Make sure we're in idle before executing         */
            ockam_vault_hal_mutex_unlock(&atecc608a_mutex, 0);
            break;
        }

        atcab_random(p_rand_num);                               /* Get a random number from the ATECC608A           */

        ockam_vault_hal_mutex_unlock(&atecc608a_mutex, 0);      /* Release the mutex                                */
    } while (0);

    return ret_val;
}


