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

    OCKAM_ERR_INVALID_PARAM                         = 0x0010,   /*!< Invalid parameter specified */
    OCKAM_ERR_INVALID_CFG                           = 0x0011,   /*!< Invalid configuration specified */
    OCKAM_ERR_INVALID_STATE                         = 0x0012,   /*!< Invalid configuration specified */
    OCKAM_ERR_INVALID_SIZE                          = 0x0013,   /*!< Invalid size specified */

    OCKAM_ERR_MEM_INSUFFICIENT                      = 0x0080,   /*!< Insufficent space available for a memory allocation */
    OCKAM_ERR_MEM_INVALID_PTR                       = 0x0081,   /*!< The specified buffer is not a managed buffer */
    OCKAM_ERR_MEM_UNAVAIL                           = 0x0082,   /*!< The requested memory size is not available */

    OCKAM_ERR_VAULT_UNINITIALIZED                   = 0x0101,   /*!< Vault needs to be initialized */
    OCKAM_ERR_VAULT_ALREADY_INIT                    = 0x0102,   /*!< Vault is already initialized */
    OCKAM_ERR_VAULT_SIZE_MISMATCH                   = 0x0103,   /*!< Specified size is invalid for the call */
    OCKAM_ERR_VAULT_KEY_FAIL                        = 0x0104,   /*!< Key failure in vault */
    OCKAM_ERR_VAULT_UNSUPPORTED_IFACE               = 0x0105,   /*!< The specified interface is not supported */
    OCKAM_ERR_VAULT_ECDH_FAIL                       = 0x0106,   /*!< ECDH failed to complete successfully */
    OCKAM_ERR_VAULT_HW_INIT_FAIL                    = 0x0107,   /*!< Hardware failed to initialize */
    OCKAM_ERR_VAULT_HW_ID_FAIL                      = 0x0108,   /*!< Hardware identification failed */
    OCKAM_ERR_VAULT_HW_ID_INVALID                   = 0x0109,   /*!< The specified hardware is not the expected hardware */
    OCKAM_ERR_VAULT_HW_UNLOCKED                     = 0x010A,   /*!< The hardware configuration is unlocked */
} OCKAM_ERR;


/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
