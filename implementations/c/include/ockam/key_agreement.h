/**
 ********************************************************************************************************
 * @file    handshake.h
 * @brief   Interface functions for establishing a secure channel and encrypting/decrypting messages
 ********************************************************************************************************
 */

#ifndef OCKAM_HANDSHAKE_H
#define OCKAM_HANDSHAKE_H

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */
#include <stdlib.h>
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define KEY_SIZE    32
#define SHA256_SIZE 32

#define KEYAGREEMENT_ERROR_TEST (OCKAM_ERROR_INTERFACE_KEYAGREEMENT | 1u)

typedef enum {
  kXXKeyAgreementFailed     = 0x0200,
  kXXKeyAgreementTestFailed = 0x0201,
} KeyAgreementError;

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 * key_establishment_xx - the handshake structure is passed to all handshake functions.
 */
typedef struct {
  uint16_t             nonce;
  uint8_t              s[KEY_SIZE];
  ockam_vault_secret_t s_secret;
  uint8_t              rs[KEY_SIZE];
  uint8_t              e[KEY_SIZE];
  ockam_vault_secret_t e_secret;
  uint8_t              re[KEY_SIZE];
  uint8_t              k[KEY_SIZE];
  ockam_vault_secret_t k_secret;
  uint8_t              ck[KEY_SIZE];
  ockam_vault_secret_t ck_secret;
  uint8_t              h[SHA256_SIZE];
  ockam_vault_secret_t ke_secret;
  ockam_vault_secret_t kd_secret;
  uint16_t             ne;
  uint16_t             nd;
  ockam_vault_t*       vault;
  ockam_reader_t*      p_reader;
  ockam_writer_t*      p_writer;
} key_establishment_xx;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

ockam_error_t key_agreement_prologue_xx(key_establishment_xx* xx);

ockam_error_t ockam_key_establish_initiator_xx(key_establishment_xx* xx);

ockam_error_t ockam_key_establish_responder_xx(key_establishment_xx* xx);

#endif
