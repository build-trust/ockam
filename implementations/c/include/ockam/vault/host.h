/**
 ********************************************************************************************************
 * @file    host.h
 * @brief   Ockam Vault Host Software Interface
 ********************************************************************************************************
 */

#ifndef OCKAM_VAULT_HOST_H_
#define OCKAM_VAULT_HOST_H_


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

#ifdef __cplusplus
extern "C" {
#endif


OCKAM_ERR ockam_vault_host_init(void *p_arg);

OCKAM_ERR ockam_vault_host_free(void);

OCKAM_ERR ockam_vault_host_random(uint8_t *p_rand_num,
                                  uint32_t rand_num_size);

OCKAM_ERR ockam_vault_host_key_gen(OCKAM_VAULT_KEY_e key_type);

OCKAM_ERR ockam_vault_host_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                       uint8_t *p_pub_key,
                                       uint32_t pub_key_size);

OCKAM_ERR ockam_vault_host_ecdh(OCKAM_VAULT_KEY_e key_type,
                                uint8_t *p_pub_key,
                                uint32_t pub_key_size,
                                uint8_t *p_pms,
                                uint32_t pms_size);


/**
 ********************************************************************************************************
 *                                          ockam_vault_host_hkdf()
 *
 * @brief   Perform HKDF in the host crypto library
 *
 * @param   p_salt[in]          Buffer for the Ockam salt value
 *
 * @param   salt_size[in]       Size of the Ockam salt value
 *
 * @param   p_ikm[in]           Buffer with the input key material for HKDF
 *
 * @param   ikm_size[in]        Size of the input key material
 *
 * @param   p_info[in]          Buffer with the optional context specific info. Can be 0.
 *
 * @param   info_size[in]       Size of the optional context specific info.
 *
 * @param   p_out[out]          Buffer for the output of the HKDF operation
 *
 * @param   out_size[in]        Size of the HKDF output buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_hkdf(uint8_t *p_salt, uint32_t salt_size,
                                uint8_t *p_ikm, uint32_t ikm_size,
                                uint8_t *p_info, uint32_t info_size,
                                uint8_t *p_out, uint32_t out_size);


/**
 ********************************************************************************************************
 *                                          ockam_vault_host_aes_gcm()
 *
 * @brief   Perform AES GCM in the mbed TLS library
 *
 * @param   mode                AES GCM Mode: Encrypt or Decrypt
 *
 * @param   p_key[in]           Buffer for the AES Key
 *
 * @param   key_size[in]        Size of the AES Key. Must be 128, 192 or 256 bits
 *
 * @param   p_iv[in]            Buffer with the initialization vector
 *
 * @param   iv_size[in]         Size of the initialization vector
 *
 * @param   p_aad[in]           Buffer with the additional authentication data (can be NULL)
 *
 * @param   aad_size[in]        Size of the additional authentication data (set to 0 if p_aad is NULL)
 *
 * @param   p_input[in]         Buffer with the input data to encrypt or decrypt
 *
 * @param   input_size[in]      Size of the input data
 *
 * @param   p_output[out]       Buffer for the output of the AES GCM operation. Can NOT be the
 *                              input buffer.
 *
 * @param   output_size[in]     Size of the output buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_host_aes_gcm(OCKAM_VAULT_AES_GCM_MODE_e mode,
                                   uint8_t *p_key, uint32_t key_size,
                                   uint8_t *p_iv, uint32_t iv_size,
                                   uint8_t *p_aad, uint32_t aad_size,
                                   uint8_t *p_tag, uint32_t tag_size,
                                   uint8_t *p_input, uint32_t input_size,
                                   uint8_t *p_output, uint32_t output_size);

#ifdef __cplusplus
}
#endif

#endif
