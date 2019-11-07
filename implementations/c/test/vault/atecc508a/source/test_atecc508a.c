/**
********************************************************************************************************
 * @file        test.atecc508a.c
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

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#include <ockam_def.h>
#include <ockam_err.h>

#include <vault/ockam_vault.h>
#include <vault/ockam_vault_hw_microchip.h>

#include <cryptoauthlib/lib/cryptoauthlib.h>
#include <cryptoauthlib/lib/atca_cfgs.h>
#include <cryptoauthlib/lib/atca_iface.h>
#include <cryptoauthlib/lib/atca_device.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define TEST_ATECC508A_PUB_KEY_SIZE                 64u
#define TEST_ATECC508A_PMS_SIZE                     32u
#define TEST_ATECC508A_RAND_NUM_SIZE                32u


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

ATCAIfaceCfg atca_iface_i2c = {
    .iface_type                 = ATCA_I2C_IFACE,
    .devtype                    = ATECC508A,
    {
        .atcai2c.slave_address  = 0x60,
        .atcai2c.bus            = 1,
        .atcai2c.baud           = 100000,
    },
    .wake_delay                 = 1500,
    .rx_retries                 = 20
};

VAULT_MICROCHIP_CFG_s atecc508a_cfg = {
    .iface                      = VAULT_MICROCHIP_IFACE_I2C,
    .iface_cfg                  = &atca_iface_i2c,
};

OCKAM_VAULT_CFG_s vault_cfg =
{
    .p_hw                       = &atecc508a_cfg,
    .p_sw                       = 0
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
 *                                             main()
 * @brief   Main point of entry

 * @param   None
 * 
 * @return  None
 * 
 ********************************************************************************************************
 */

void main (void)
{
    OCKAM_ERR err;
    uint8_t i;
    uint8_t rand_num[TEST_ATECC508A_RAND_NUM_SIZE];
    uint8_t key_static[TEST_ATECC508A_PUB_KEY_SIZE];
    uint8_t key_ephemeral[TEST_ATECC508A_PUB_KEY_SIZE];
    uint8_t pms_static[TEST_ATECC508A_PMS_SIZE];
    uint8_t pms_ephemeral[TEST_ATECC508A_PMS_SIZE];


    /* ---------- */
    /* Vault Init */
    /* ---------- */

    err = ockam_vault_init((void*) &atecc508a_cfg);             /* Initialize Vault                                     */
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vauilt Init failed\r\n");
    }

    /* ------------------------ */
    /* Random Number Generation */
    /* ------------------------ */

    err = ockam_vault_random(&rand_num,                         /* Generate a random number                             */
                             TEST_ATECC508A_RAND_NUM_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault Random failed\r\n");
    }

    printf("Random Number Generation Output:\r\n");

    for(i = 1; i <= TEST_ATECC508A_RAND_NUM_SIZE; i++) {
        printf("%02X ", rand_num[i-1]);
        if(i % 8 == 0) {
            printf("\r\n");
        }
    }

    /* -------------- */
    /* Key Generation */
    /* -------------- */

    //TODO base this off config??

    err = ockam_vault_key_gen(OCKAM_VAULT_KEY_STATIC,           /* Generate a static key                                */
                              &key_static[0],
                              TEST_ATECC508A_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault Static Key Generate Failed\r\n");
    }

    err = ockam_vault_key_gen(OCKAM_VAULT_KEY_EPHEMERAL,        /* Generate an ephemrmal key                            */ 
                              &key_static[0],
                              TEST_ATECC508A_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault Ephemeral Key Generate Failed\r\n");
    }

    /* ------------ */
    /* Key Retrival */
    /* ------------ */

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_STATIC,       /* Get the static public key                            */
                                  &key_static[0],
                                  TEST_ATECC508A_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault Get Static Public Key Failed\r\n");
    }

    err = ockam_vault_key_get_pub(OCKAM_VAULT_KEY_EPHEMERAL,    /* Get the ephemrmal public key                         */
                                  &key_ephemeral[0],
                                  TEST_ATECC508A_PUB_KEY_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault Get Ephemeral Public Key Failed\r\n");
    }

    /* ----------------- */
    /* ECDH Calculations */
    /* ----------------- */

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_STATIC,              /* Calculate ECDH with static private and ephemeral pub */
                           &key_ephemeral[0],
                           TEST_ATECC508A_PUB_KEY_SIZE,
                           &pms_static[0],
                           TEST_ATECC508A_PMS_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault ECDH Failed\r\n");
    }

    err = ockam_vault_ecdh(OCKAM_VAULT_KEY_EPHEMERAL,          /* Calculate ECDH with static private and ephemeral pub */
                           &key_static[0],
                           TEST_ATECC508A_PUB_KEY_SIZE,
                           &pms_ephemeral[0],
                           TEST_ATECC508A_PMS_SIZE);
    if(err != OCKAM_ERR_NONE) {
        printf("Error: Ockam Vault ECDH Failed\r\n");
    }

    for(i = 0; i < TEST_ATECC508A_PMS_SIZE; i++) {
        if(pms_static[i] != pms_ephemeral[i]) {
            printf("Error: Ockam Vault PMS do not match!\r\n");
            break;
        }
    }

    return;
}

