/**
 ********************************************************************************************************
 * @file        error.h
 * @brief   
 ********************************************************************************************************
 */

#ifndef OCKAM_ERROR_H_
#define OCKAM_ERROR_H_


/*
 ********************************************************************************************************
 * @defgroup    OCKAM_ERROR OCKAM_ERROR_API
 * @ingroup     OCKAM
 * @brief       OCKAM_ERROR_API
 *
 * @addtogroup  OCKAM_ERROR
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
    OCKAM_ERR_NONE                                    = 0x0000, /*!< No error                                         */

    OCKAM_ERR_INVALID_PARAM                           = 0x0010, /*!< Invalid parameter specified                      */
    OCKAM_ERR_INVALID_CFG                             = 0x0011, /*!< Invalid configuration specified                  */
    OCKAM_ERR_INVALID_SIZE                            = 0x0013, /*!< Invalid size specified                           */

    OCKAM_ERR_MEM_INSUFFICIENT                        = 0x0080, /*!< Insufficent space for a memory allocation        */
    OCKAM_ERR_MEM_INVALID_PTR                         = 0x0081, /*!< The specified buffer is not a managed buffer     */
    OCKAM_ERR_MEM_UNAVAIL                             = 0x0082, /*!< The requested memory size is not available       */

    OCKAM_ERR_VAULT_UNINITIALIZED                     = 0x0101, /*!< Vault needs to be initialized                    */
    OCKAM_ERR_VAULT_ALREADY_INIT                      = 0x0102, /*!< Vault is already initialized                     */
    OCKAM_ERR_VAULT_SIZE_MISMATCH                     = 0x0103, /*!< Specified size is invalid for the call           */

    OCKAM_ERR_VAULT_HW_INIT_FAIL                      = 0x0201, /*!< Hardware failed to initialize                    */
    OCKAM_ERR_VAULT_HW_KEY_FAIL                       = 0x0202, /*!< Key failure in vault                             */
    OCKAM_ERR_VAULT_HW_ECDH_FAIL                      = 0x0203, /*!< ECDH failed to complete successfully             */
    OCKAM_ERR_VAULT_HW_HKDF_FAIL                      = 0x0204, /*!< HKDF failed to complete successfully             */
    OCKAM_ERR_VAULT_HW_AES_FAIL                       = 0x0205, /*!< AES failed to complete successfully              */
    OCKAM_ERR_VAULT_HW_ID_FAIL                        = 0x0206, /*!< Hardware identification failed                   */
    OCKAM_ERR_VAULT_HW_ID_INVALID                     = 0x0207, /*!< Specified hardware is not the expected hardware  */
    OCKAM_ERR_VAULT_HW_UNLOCKED                       = 0x0208, /*!< The hardware configuration is unlocked           */
    OCKAM_ERR_VAULT_HW_UNSUPPORTED_IFACE              = 0x0209, /*!< The specified interface is not supported         */

    OCKAM_ERR_VAULT_SW_INIT_FAIL                      = 0x0301, /*!< Software library failed to initialize            */
    OCKAM_ERR_VAULT_SW_KEY_FAIL                       = 0x0302, /*!< Key failure in software                          */
    OCKAM_ERR_VAULT_SW_ECDH_FAIL                      = 0x0303, /*!< ECDH failed to complete successfully             */
    OCKAM_ERR_VAULT_SW_HKDF_FAIL                      = 0x0304, /*!< HKDF failed to complete successfully             */
    OCKAM_ERR_VAULT_SW_AES_FAIL                       = 0x0305  /*!< AES failed to complete successfully              */
} OCKAM_ERR;


/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
