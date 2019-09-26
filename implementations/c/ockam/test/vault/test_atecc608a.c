/**
 ********************************************************************************************************
 * @file        test.atecc608a.c
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

#include <vault/inc/ockam_err.h>
#include <vault/inc/ockam_vault.h>
#include <vault/inc/ockam_vault_port.h>

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
 *                                             main()
 * @brief   Main point of entry

 * @param  
 * 
 * @return
 * 
 ********************************************************************************************************
 */

void main (void)
{
    OCKAM_ERR err;
    uint8_t rand_num[32];


    err = ockam_vault_port_init(0);
    err = ockam_vault_port_random(&rand_num);

    printf("Random number: %32u", rand_num);

    return;
}

