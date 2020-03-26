/**
 ********************************************************************************************************
 * @file    handshake.h
 * @brief   Interface functions for establishing a secure channel and
 *encrypting/decrypting messages
 ********************************************************************************************************
 */

#ifndef HANDSHAKE_LOCAL_H
#define HANDSHAKE_LOCAL_H

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES *
 ********************************************************************************************************
 */
#include <stdlib.h>

#include "ockam/error.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "ockam/key_agreement.h"

/*
 ********************************************************************************************************
 *                                                DEFINES *
 ********************************************************************************************************
 */

#define PROTOCOL_NAME "Noise_XX_25519_AESGCM_SHA256"
#define PROTOCOL_NAME_SIZE 28
#define MAX_TRANSMIT_SIZE 2048
#define TAG_SIZE 16
#define VECTOR_SIZE 12

#define DEFAULT_IP_ADDRESS "127.0.0.1"
#define DEFAULT_IP_PORT 8001

/*
 ********************************************************************************************************
 *                                               DATA TYPES *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES *
 ********************************************************************************************************
 */
OckamError xx_handshake_prologue(KeyEstablishmentXX *p_h);
void print_uint8_str(uint8_t *p, uint16_t size, char *msg);
void string_to_hex(char *hexstring, uint8_t *val, uint32_t *p_bytes);
void mix_hash(KeyEstablishmentXX *p_handshake, uint8_t *p_bytes, uint16_t b_length);

OckamError XXResponderM1Process(KeyEstablishmentXX *p_h, uint8_t *p_m1, uint16_t m1_size);
OckamError XXResponderM2Make(KeyEstablishmentXX *p_h, uint8_t *p_msg, uint16_t msg_size, uint16_t *p_bytesWritten);
OckamError XXResponderM3Process(KeyEstablishmentXX *p_h, uint8_t *p_m3, uint16_t m3_size);
OckamError XXResponderEpilogue(KeyEstablishmentXX *p_h);
OckamError XXInitiatorM1Make(KeyEstablishmentXX *p_h, uint8_t *p_sendBuffer, uint16_t buffer_length,
                             uint16_t *p_transmit_size);
OckamError XXInitiatorM2Process(KeyEstablishmentXX *p_h, uint8_t *p_recv, uint16_t recv_size);
OckamError XXInitiatorM3Make(KeyEstablishmentXX *p_h, uint8_t *p_msg, uint16_t *p_msg_size);
OckamError XXInitiatorEpilogue(KeyEstablishmentXX *p_h);
OckamError XXEncrypt(KeyEstablishmentXX *xx, uint8_t *payload, uint32_t payload_size, uint8_t *msg, uint16_t msg_length,
                     uint16_t *msg_size);
OckamError XXDecrypt(KeyEstablishmentXX *xx, uint8_t *payload, uint32_t payload_size, uint8_t *msg, uint16_t msg_length,
                     uint32_t *payload_bytes);
OckamError GetIpInfo(int argc, char *argv[], OckamInternetAddress *p_address);
OckamError make_vector(uint64_t nonce, uint8_t *p_vector);
OckamError HkdfDh(KeyEstablishmentXX *xx, uint8_t *hkdf1, uint16_t hkdf1_size, OckamVaultKey dh_key, uint8_t *dh2,
                  uint16_t dh2_size, uint16_t out_size, uint8_t *out_1, uint8_t *out_2);

#endif
