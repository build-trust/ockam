/**
 ********************************************************************************************************
 * @file        ockam_vault.h
 * @author      Mark Mulrooney <mark@ockam.io>
 * @copyright   Copyright (c) 2019, Ockam Inc.
 * @brief   
 ********************************************************************************************************
 */

#ifndef OCKAM_VAULT_H_
#define OCKAM_VAULT_H_

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam_def.h>
#include <ockam_err.h>


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

/**
 *******************************************************************************
 * @enum    OCKAM_VAULT_KEY_e
 * @brief   Key types
 *******************************************************************************
 */
typedef enum {
    OCKAM_VAULT_KEY_STATIC  = 0,                                /*!< Static key                                         */
    OCKAM_VAULT_KEY_EPHEMERAL,                                  /*!< Ephemeral key                                      */
    MAX_OCKAM_VAULT_KEY                                         /*!< Total number of key types supported                */
} OCKAM_VAULT_KEY_e;


/**
 *******************************************************************************
 * @enum    OCKAM_VAULT_CFG_FN_e
 * @brief   
 *******************************************************************************
 */
typedef enum {
    OCKAM_VAULT_CFG_FN_HW,                                      /*!<  Vault operation is performed on the hardware port */
    OCKAM_VAULT_CFG_FN_CRYPTO,                                  /*!<  Vault operation is performed in the crypto lib    */
    OCKAM_VAULT_CFG_FN_BOTH                                     /*!<  Vault operation is performed on port and crypto   */
} OCKAM_VAULT_CFG_FN_e;


/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */


/**
 *******************************************************************************
 * @struct  OCKAM_VAULT_CFG_s
 * @brief   
 *******************************************************************************
 */

typedef struct {
    OCKAM_VAULT_CFG_FN_e init;                                  /*!<  Vault Init Functions Config                       */
    OCKAM_VAULT_CFG_FN_e random;                                /*!<  Vault Random Function Config                      */
    OCKAM_VAULT_CFG_FN_e key;                                   /*!<  Vault Key Functions Config                        */
    OCKAM_VAULT_CFG_FN_e ecdh;                                  /*!<  Vault ECDH Functions Config                       */
    OCKAM_VAULT_CFG_FN_e hkdf;                                  /*!<  Vault HDKF Functions Config                       */
    OCKAM_VAULT_CFG_FN_e aes_gcm;                               /*!<  Vault AES GMC Functions Config                    */
} OCKAM_VAULT_CFG_FN_s;



/**
 *******************************************************************************
 * @struct  OCKAM_VAULT_CFG_s
 * @brief   
 *******************************************************************************
 */
typedef struct {
    void* p_hw;                                                 /*!<  Hardware specific configuration                   */
    void* p_sw;                                                 /*!<  Software library specific configuration           */
} OCKAM_VAULT_CFG_s;


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

extern const OCKAM_VAULT_CFG_FN_s ockam_vault_cfg_fn;


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


OCKAM_ERR ockam_vault_init(OCKAM_VAULT_CFG_s *p_cfg);

OCKAM_ERR ockam_vault_random(uint8_t *p_rand_num,
                             uint32_t rand_num_size);

OCKAM_ERR ockam_vault_key_gen(OCKAM_VAULT_KEY_e key_type,
                              uint8_t *p_key_pub,
                              uint32_t key_pub_size);

OCKAM_ERR ockam_vault_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                  uint8_t *p_pub_key,
                                  uint32_t pub_key_size);

OCKAM_ERR ockam_vault_ecdh(OCKAM_VAULT_KEY_e key_type,
                           uint8_t *p_pub_key,
                           uint32_t pub_key_size,
                           uint8_t *p_pms,
                           uint32_t pms_size);

OCKAM_ERR ockam_vault_hkdf(uint8_t *p_salt,
                           uint32_t salt_size,
                           uint8_t *p_ikm,
                           uint32_t ikm_size,
                           uint8_t *p_info,
                           uint32_t info_size,
                           uint8_t *p_out,
                           uint32_t out_size);
#endif
