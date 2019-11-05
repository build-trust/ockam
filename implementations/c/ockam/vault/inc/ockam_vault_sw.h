/**
 ********************************************************************************************************
 * @file        ockam_vault_sw.h
 * @author      Mark Mulrooney <mark@ockam.io>
 * @copyright   Copyright (c) 2019, Ockam Inc.
 * @brief   
 ********************************************************************************************************
 */

#ifndef OCKAM_VAULT_SW_H_ 
#define OCKAM_VAULT_SW_H_


/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <common/inc/ockam_def.h>
#include <common/inc/ockam_err.h>

#include <ockam_vault_cfg.h>


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


#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_CFG_SW)
OCKAM_ERR ockam_vault_sw_init(void *p_arg);

OCKAM_ERR ockam_vault_sw_free(void);
#endif

#if(OCKAM_VAULT_CFG_RAND & OCKAM_VAULT_CFG_SW)
OCKAM_ERR ockam_vault_sw_random(uint8_t *p_rand_num,
                                uint32_t rand_num_size);
#endif

#if(OCKAM_VAULT_CFG_KEY_ECHD & OCKAM_VAULT_CFG_SW)
OCKAM_ERR ockam_vault_sw_key_gen(OCKAM_VAULT_KEY_e key_type,
                                 uint8_t *p_pub_key,
                                 uint32_t pub_key_size);

OCKAM_ERR ockam_vault_sw_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                     uint8_t *p_pub_key,
                                     uint32_t pub_key_size);

OCKAM_ERR ockam_vault_sw_ecdh(OCKAM_VAULT_KEY_e key_type,
                              uint8_t *p_pub_key,
                              uint32_t pub_key_size,
                              uint8_t *p_pms,
                              uint32_t pms_size);
#endif

#if(OCKAM_VAULT_CFG_HKDF & OCKAM_VAULT_CFG_SW)
OCKAM_ERR ockam_vault_sw_hkdf(uint8_t *p_salt,
                              uint32_t salt_size,
                              uint8_t *p_ikm,
                              uint32_t ikm_size,
                              uint8_t *p_info,
                              uint32_t info_size,
                              uint8_t *p_out,
                              uint32_t out_size);
#endif

#ifdef __cplusplus
}
#endif

#endif
