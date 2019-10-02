/**
 ********************************************************************************************************
 * @file        ockam_err.h
 * @author      Mark Mulrooney <mark@ockam.io>
 * @copyright   Copyright (c) 2019, Ockam Inc.
 * @brief   
 ********************************************************************************************************
 */

#ifndef OCKAM_ERR_H_
#define OCKAM_ERR_H_


/*
 ********************************************************************************************************
 * @defgroup    OCKAM_ERR OCKAM_ERR_API
 * @ingroup     OCKAM
 * @brief       OCKAM_ERR_API
 *
 * @addtogroup  OCKAM_ERR
 * @{
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @enum    OCKAM_ERR_e
 * @brief   The Ockam error enum values
 *******************************************************************************
 */
typedef enum {
    OCKAM_ERR_NONE                                  = 0x0000,   /*!< No error */

    OCKAM_ERR_VAULT_UNINITIALIZED                   = 0x0101,   /*!< Vault needs to be initialized */
    OCKAM_ERR_VAULT_ALREADY_INIT                    = 0x0102,   /*!< Vault is already initialized */
    OCKAM_ERR_VAULT_SIZE_MISMATCH                   = 0x0103,   /*!< Specified size is invalid for the call */
    OCKAM_ERR_VAULT_HW_INIT_FAIL                    = 0x0104,   /*!< Hardware failed to initialize */
} OCKAM_ERR;


/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
