/**
 ********************************************************************************************************
 * @file    vault.h
 * @brief   Vault interface for the Ockam Library
 ********************************************************************************************************
 */

#ifndef OCKAM_VAULT_H_
#define OCKAM_VAULT_H_


/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam/define.h>
#include <ockam/error.h>


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
 * @brief   Support key types in Ockam Vault
 *******************************************************************************
 */

typedef enum {
    OCKAM_VAULT_KEY_STATIC      = 0,                            /*!< Static key                                       */
    OCKAM_VAULT_KEY_EPHEMERAL,                                  /*!< Ephemeral key                                    */
    MAX_OCKAM_VAULT_KEY                                         /*!< Total number of key types supported              */
} OCKAM_VAULT_KEY_e;


/**
 *******************************************************************************
 * @enum    OCKAM_VAULT_AES_GCM_MODE_e
 * @brief   Specifies the mode of operation for AES GCM
 *******************************************************************************
 */

typedef enum {
    OCKAM_VAULT_AES_GCM_MODE_ENCRYPT = 0,                       /*!< Encrypt using AES GCM                            */
    OCKAM_VAULT_AES_GCM_MODE_DECRYPT                            /*!< Decrypt using AES GCM                            */
} OCKAM_VAULT_AES_GCM_MODE_e;


/**
 *******************************************************************************
 * @enum    OCKAM_VAULT_EC_e
 * @brief   The elliptic curve vault will support
 *******************************************************************************
 */
typedef enum {
    OCKAM_VAULT_EC_P256 = 0,                                   /*!< NIST P-256/SECP256R1                             */
    OCKAM_VAULT_EC_CURVE25519                                  /*!< Curve 25519                                      */
} OCKAM_VAULT_EC_e;


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
    void* p_tpm;                                                /*!<  TPM specific configuration                      */
    void* p_host;                                               /*!<  Host software library specific configuration    */
    OCKAM_VAULT_EC_e ec;                                        /*!< The type of EC Key supported by vault            */
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

OCKAM_ERR ockam_vault_random(uint8_t *p_rand_num, uint32_t rand_num_size);

OCKAM_ERR ockam_vault_key_gen(OCKAM_VAULT_KEY_e key_type);

OCKAM_ERR ockam_vault_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                  uint8_t *p_pub_key, uint32_t pub_key_size);

OCKAM_ERR ockam_vault_ecdh(OCKAM_VAULT_KEY_e key_type,
                           uint8_t *p_pub_key, uint32_t pub_key_size,
                           uint8_t *p_pms, uint32_t pms_size);

OCKAM_ERR ockam_vault_sha256(uint8_t *p_msg, uint16_t msg_size,
                             uint8_t *p_digest, uint8_t digest_size);

OCKAM_ERR ockam_vault_hkdf(uint8_t *p_salt, uint32_t salt_size,
                           uint8_t *p_ikm, uint32_t ikm_size,
                           uint8_t *p_info, uint32_t info_size,
                           uint8_t *p_out, uint32_t out_size);

OCKAM_ERR ockam_vault_aes_gcm(OCKAM_VAULT_AES_GCM_MODE_e mode,
                              uint8_t *p_key, uint32_t key_size,
                              uint8_t *p_iv, uint32_t iv_size,
                              uint8_t *p_aad, uint32_t aad_size,
                              uint8_t *p_tag, uint32_t tag_size,
                              uint8_t *p_input, uint32_t input_size,
                              uint8_t *p_output, uint32_t output_size);

OCKAM_ERR ockam_vault_aes_gcm_encrypt(uint8_t *p_key, uint32_t key_size,
                                      uint8_t *p_iv, uint32_t iv_size,
                                      uint8_t *p_aad, uint32_t aad_size,
                                      uint8_t *p_tag, uint32_t tag_size,
                                      uint8_t *p_input, uint32_t input_size,
                                      uint8_t *p_output, uint32_t output_size);

OCKAM_ERR ockam_vault_aes_gcm_decrypt(uint8_t *p_key, uint32_t key_size,
                                      uint8_t *p_iv, uint32_t iv_size,
                                      uint8_t *p_aad, uint32_t aad_size,
                                      uint8_t *p_tag, uint32_t tag_size,
                                      uint8_t *p_input, uint32_t input_size,
                                      uint8_t *p_output, uint32_t output_size);

#endif
