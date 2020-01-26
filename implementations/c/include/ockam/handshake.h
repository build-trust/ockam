#ifndef OCKAM_HANDSHAKE_H
#define OCKAM_HANDSHAKE_H

#include <stdlib.h>
#include "ockam/error.h"
#include "ockam/vault.h"

#define KEY_SIZE 32
#define NAME_SIZE 28
#define SHA256_SIZE 32
#define NAME "Noise_XX_25519_AESGCM_SHA256"
#define MAX_TRANSMIT_SIZE 2048
#define DHLEN 32
#define TAG_SIZE 16
#define VECTOR_SIZE 12
#define EPI_STRING_SIZE 30
#define EPI_BYTE_SIZE 15
#define EPI_INITIATOR "7375626d6172696e6579656c6c6f77"
#define EPI_RESPONDER "79656c6c6f777375626d6172696e65"

#define INITIATOR_STATIC    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
#define RESPONDER_STATIC    "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
#define INITIATOR_EPH       "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"
#define RESPONDER_EPH       "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60"


typedef struct  {

	uint64_t    nonce;
	uint8_t     s[KEY_SIZE];
	uint8_t     rs[KEY_SIZE];
	uint8_t     e[KEY_SIZE];
	uint8_t     re[KEY_SIZE];
	uint8_t     k[KEY_SIZE];
	uint8_t     ck[SHA256_SIZE];
	uint8_t     h[SHA256_SIZE];
	uint8_t     ke[KEY_SIZE];
	uint8_t     kd[KEY_SIZE];
	uint64_t    ne;
	uint64_t    nd;
} HANDSHAKE;

OCKAM_ERR decrypt( HANDSHAKE* p_h,
		uint8_t* p_payload, uint32_t payload_size, uint8_t* p_msg, uint16_t msg_length, uint32_t* p_payload_bytes );

OCKAM_ERR encrypt( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                   uint8_t* p_msg, uint16_t msg_length, uint16_t* p_msg_size );

void print_uint8_str( uint8_t* p, uint16_t size, char* msg );
void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes );

OCKAM_ERR responder_m1_process( HANDSHAKE* p_h, uint8_t* p_m1, uint16_t m1_size );
OCKAM_ERR responder_m2_make( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                             uint8_t* p_msg, uint16_t msg_size, uint16_t* p_bytes_written );
OCKAM_ERR responder_m3_process( HANDSHAKE* p_h, uint8_t* p_m3, uint16_t m3_size );
OCKAM_ERR initiator_m1_make( HANDSHAKE* p_h, uint8_t* p_prologue, uint16_t prologue_length,
                             uint8_t* p_payload, uint16_t payload_length, uint8_t* p_send_buffer, uint16_t buffer_length,
                             uint16_t* p_transmit_size );
OCKAM_ERR initiator_m2_process( HANDSHAKE* p_h, uint8_t* p_recv, uint16_t recv_size );
OCKAM_ERR initiator_m3_make( HANDSHAKE* p_h, uint8_t* p_msg, uint16_t* p_msg_size );
OCKAM_ERR initiator_epilogue( HANDSHAKE* p_h );
#endif
