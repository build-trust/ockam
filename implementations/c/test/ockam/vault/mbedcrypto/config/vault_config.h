/**
 ********************************************************************************************************
 * @file        vault_config.h
 * @brief
 ********************************************************************************************************
 */

#ifndef VAULT_CONFIG_H_
#define VAULT_CONFIG_H_


/*
 ********************************************************************************************************
 *                                               INCLUDES                                               *
 ********************************************************************************************************
 */

#include <ockam/vault/define.h>


/*
 ********************************************************************************************************
 *                                         Function Configuration                                       *
 ********************************************************************************************************
 */


#define OCKAM_VAULT_CFG_INIT               OCKAM_VAULT_HOST_MBEDCRYPTO

#define OCKAM_VAULT_CFG_RAND               OCKAM_VAULT_HOST_MBEDCRYPTO

#define OCKAM_VAULT_CFG_KEY_ECDH           OCKAM_VAULT_HOST_MBEDCRYPTO

#define OCKAM_VAULT_CFG_SHA256             OCKAM_VAULT_HOST_MBEDCRYPTO

#define OCKAM_VAULT_CFG_HKDF               OCKAM_VAULT_HOST_MBEDCRYPTO

#define OCKAM_VAULT_CFG_AES_GCM            OCKAM_VAULT_HOST_MBEDCRYPTO


#endif
