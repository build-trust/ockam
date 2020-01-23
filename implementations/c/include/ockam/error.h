/**
 ********************************************************************************************************
 * @file    error.h
 * @brief   Ockam Error Codes
 *
 * This file contains all error codes used across all modules in the Ockam library
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

    OCKAM_ERR_INVALID_PARAM                           = 0x0011, /*!< Invalid parameter specified                      */
    OCKAM_ERR_INVALID_CFG                             = 0x0012, /*!< Invalid configuration specified                  */
    OCKAM_ERR_INVALID_SIZE                            = 0x0013, /*!< Invalid size specified                           */
    OCKAM_ERR_UNIMPLEMENTED                           = 0x0014, /*!< Function has not yet been implemented            */

    OCKAM_ERR_MEM_INSUFFICIENT                        = 0x0080, /*!< Insufficent space for a memory allocation        */
    OCKAM_ERR_MEM_INVALID_PTR                         = 0x0081, /*!< The specified buffer is not a managed buffer     */
    OCKAM_ERR_MEM_UNAVAIL                             = 0x0082, /*!< The requested memory size is not available       */

    OCKAM_ERR_VAULT_UNINITIALIZED                     = 0x0101, /*!< Vault needs to be initialized                    */
    OCKAM_ERR_VAULT_ALREADY_INIT                      = 0x0102, /*!< Vault is already initialized                     */
    OCKAM_ERR_VAULT_SIZE_MISMATCH                     = 0x0103, /*!< Specified size is invalid for the call           */
    OCKAM_ERR_VAULT_INVALID_KEY_SIZE                  = 0x0104, /*!< Supplied keysize is invalid for call             */
    OCKAM_ERR_VAULT_INVALID_BUFFER                    = 0x0105, /*!< Supplied buffer is null                          */
    OCKAM_ERR_VAULT_INVALID_BUFFER_SIZE               = 0x0106, /*!< Supplied buffer size is invalid for call         */

    OCKAM_ERR_VAULT_TPM_INIT_FAIL                     = 0x0201, /*!< TPM failed to initialize                         */
    OCKAM_ERR_VAULT_TPM_RAND_FAIL                     = 0x0202, /*!< Random number generator failure                  */
    OCKAM_ERR_VAULT_TPM_KEY_FAIL                      = 0x0203, /*!< Key failure in vault                             */
    OCKAM_ERR_VAULT_TPM_ECDH_FAIL                     = 0x0204, /*!< ECDH failed to complete successfully             */
    OCKAM_ERR_VAULT_TPM_SHA256_FAIL                   = 0x0205, /*!< SHA256 unable to complete                        */
    OCKAM_ERR_VAULT_TPM_HKDF_FAIL                     = 0x0206, /*!< HKDF failed to complete successfully             */
    OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL                  = 0x0207, /*!< AES failed to complete successfully              */
    OCKAM_ERR_VAULT_TPM_ID_FAIL                       = 0x0208, /*!< Hardware identification failed                   */
    OCKAM_ERR_VAULT_TPM_ID_INVALID                    = 0x0209, /*!< Specified hardware is not the expected hardware  */
    OCKAM_ERR_VAULT_TPM_UNLOCKED                      = 0x020A, /*!< The hardware configuration is unlocked           */
    OCKAM_ERR_VAULT_TPM_UNSUPPORTED_IFACE             = 0x020B, /*!< The specified interface is not supported         */
    OCKAM_ERR_VAULT_TPM_AES_GCM_DECRYPT_INVALID       = 0x020C, /*!< AES GCM tag invalid for decryption               */

    OCKAM_ERR_VAULT_HOST_INIT_FAIL                    = 0x0301, /*!< Host software library failed to initialize       */
    OCKAM_ERR_VAULT_HOST_RAND_FAIL                    = 0x0302, /*!< Random number failed to generate on host         */
    OCKAM_ERR_VAULT_HOST_KEY_FAIL                     = 0x0303, /*!< Key failure in software                          */
    OCKAM_ERR_VAULT_HOST_ECDH_FAIL                    = 0x0304, /*!< ECDH failed to complete successfully             */
    OCKAM_ERR_VAULT_HOST_SHA256_FAIL                  = 0x0305, /*!< SHA256 failed to complete sucessfully            */
    OCKAM_ERR_VAULT_HOST_HKDF_FAIL                    = 0x0306, /*!< HKDF failed to complete successfully             */
    OCKAM_ERR_VAULT_HOST_AES_FAIL                     = 0x0307  /*!< AES failed to complete successfully              */
} OCKAM_ERR;


/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
