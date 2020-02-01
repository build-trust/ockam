/**
 ********************************************************************************************************
 * @file    handshake.h
 * @brief   Interface functions for establishing a secure channel and encrypting/decrypting messages
 ********************************************************************************************************
 */

#ifndef HANDSHAKE_LOCAL_H
#define HANDSHAKE_LOCAL_H


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

#define PROTOCOL_NAME "Noise_XX_25519_AESGCM_SHA256"
#define PROTOCOL_NAME_SIZE 28
#define MAX_TRANSMIT_SIZE 2048
#define DHLEN 32
#define TAG_SIZE 16
#define VECTOR_SIZE 12

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
OCKAM_ERR xx_handshake_prologue( XX_HANDSHAKE* p_h );
void print_uint8_str( uint8_t* p, uint16_t size, char* msg );
void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes );
void mix_hash( XX_HANDSHAKE* p_handshake,  uint8_t* p_bytes, uint16_t b_length );

OCKAM_ERR xx_responder_m1_process( XX_HANDSHAKE* p_h, uint8_t* p_m1, uint16_t m1_size );
OCKAM_ERR xx_responder_m2_make( XX_HANDSHAKE* p_h, uint8_t* p_msg, uint16_t msg_size, uint16_t* p_bytes_written );
OCKAM_ERR xx_responder_m3_process( XX_HANDSHAKE* p_h, uint8_t* p_m3, uint16_t m3_size );
OCKAM_ERR xx_responder_epilogue( XX_HANDSHAKE* p_h );
OCKAM_ERR xx_initiator_m1_make( XX_HANDSHAKE* p_h, uint8_t* p_send_buffer, uint16_t buffer_length, uint16_t* p_transmit_size );
OCKAM_ERR xx_initiator_m2_process( XX_HANDSHAKE* p_h, uint8_t* p_recv, uint16_t recv_size );
OCKAM_ERR xx_initiator_m3_make( XX_HANDSHAKE* p_h, uint8_t* p_msg, uint16_t* p_msg_size );
OCKAM_ERR xx_initiator_epilogue( XX_HANDSHAKE* p_h );
OCKAM_ERR make_vector( uint64_t nonce, uint8_t* p_vector );
OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 );

#endif
