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

#define KEY_SIZE 32
#define SHA256_SIZE 32

typedef enum {
  kXXKeyAgreementFailed = 0x0200,
  kXXKeyAgreementTestFailed = 0x0201,
} KeyAgreementError;

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 * KeyEstablishmentXX - the handshake structure is passed to all handshake functions.
 */
typedef struct {
  uint64_t nonce;
  uint8_t s[KEY_SIZE];
  uint8_t rs[KEY_SIZE];
  uint8_t e[KEY_SIZE];
  uint8_t re[KEY_SIZE];
  uint8_t k[KEY_SIZE];
  uint8_t ck[KEY_SIZE];
  uint8_t h[SHA256_SIZE];
  uint8_t ke[KEY_SIZE];
  uint8_t kd[KEY_SIZE];
  uint64_t ne;
  uint64_t nd;
  const OckamVault* vault;
  OckamVaultCtx* vault_ctx;
  const OckamTransport* transport;
  OckamTransportCtx transport_ctx;
} KeyEstablishmentXX;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/**
 *********************************************************************************************************
 *                                      OckamKeyInitializeXX
 * @param xx [in] - Pointer to 0-initialized KeyEstablishmentXX
 * @param vault [in] - Pointer to initialize OckamVault
 * @return
 *********************************************************************************************************
 */
OckamError OckamKeyInitializeXX(KeyEstablishmentXX* xx, const OckamVault* vault, OckamVaultCtx* vault_ctx,
                                const OckamTransport* transport, OckamTransportCtx transport_ctx);

OckamError KeyEstablishPrologueXX(KeyEstablishmentXX* xx);

/**
 * ********************************************************************************************************
 *                                      OckamKeyEstablishResponderXX
 *
 * @param connection [in] - Initialize OCKAM_CONNECTION instance (must be connected)
 * @param p_h [in/out] - pointer to the KeyEstablishmentXX structure. Should be 0-initialized prior to calling,
 *                      and not modified thereafter.
 * @return [out] - OckamError_NONE on success
 *
 * ********************************************************************************************************
 */
OckamError OckamKeyEstablishResponderXX(const OckamVault* vault, OckamVaultCtx* vault_ctx,
                                        const OckamTransport* transport, OckamTransportCtx transport_ctx,
                                        KeyEstablishmentXX* xx);
/**
 * ********************************************************************************************************
 *                                      ockam_initiator_handshake
 *
 * @param transport [in] - Initialize OCKAM_CONNECTION instance (must be connected)
 * @param xx [in/out] - pointer to the KeyEstablishmentXX structure. Should be initialized prior to calling,
 *                      and not modified thereafter.
 * @return [out] - OckamError_NONE on success
 *
 * ********************************************************************************************************
 */
OckamError OckamKeyEstablishInitiatorXX(const OckamVault* vault, OckamVaultCtx* vault_ctx,
                                        const OckamTransport* transport, OckamTransportCtx transport_ctx,
                                        KeyEstablishmentXX* xx);

/**
 * ********************************************************************************************************
 *                                      decrypt
 *
 * @param p_h [in] - pointer to handshake struct, post-hanshake
 * @param p_payload [out] - pointer to payload buffer
 * @param payload_size [in] - size of payload buffer
 * @param p_msg [in] - pointer to raw buffer as received from transport
 * @param msg_length [in] - number of bytes received from transport
 * @param p_payload_bytes [out] - number of bytes decrypted into p_payload
 * @return [out] - OckamError_NONE on success
 *
 * ********************************************************************************************************
 */
OckamError xx_decrypt(KeyEstablishmentXX* p_h, uint8_t* p_payload, uint32_t payload_size, uint8_t* p_msg,
                      uint16_t msg_length, uint32_t* p_payload_bytes);

/**
 * ********************************************************************************************************
 *                                      encrypt
 *
 * @param p_h [in] - pointer to handshake struct, post-hanshake
 * @param p_payload [in] - pointer to payload buffer
 * @param payload_size [in] - number of bytes to encrypt
 * @param p_msg [in] - pointer to buffer that will be handed to transport
 * @param msg_length [in] - size of p_msg buffer
 * @param p_msg_size [out] - number of bytes written to p_msg, this will be the number of bytes to send.
 *                          Note: this will be larger than the payload size, to account for encryption data
 * @return [out] - OckamError_NONE on success
 *
 * ********************************************************************************************************
 */
OckamError xx_encrypt(KeyEstablishmentXX* p_h, uint8_t* p_payload, uint32_t payload_size, uint8_t* p_msg,
                      uint16_t msg_length, uint16_t* p_msg_size);

void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes);

#endif
