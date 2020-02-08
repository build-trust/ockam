/**
 ********************************************************************************************************
 * @file    xx_responder.c
 * @brief   Interface functions for xx handshake responder
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <string.h>
#include "ockam/vault.h"
#include "ockam/error.h"
#include "ockam/syslog.h"
#include "ockam/handshake.h"
#include "handshake_local.h"
#include "handshake_test.h"


/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_xx_responder_handshake( OCKAM_TRANSPORT_CONNECTION connection, XX_HANDSHAKE* p_h )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
	uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
	uint16_t                        transmit_size = 0;
	uint16_t                        bytes_received = 0;

	/* Initialize handshake struct and generate initial static & ephemeral keys */
	status = xx_handshake_prologue( p_h );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "Failed handshake prologue");
		goto exit_block;
	}

	/* Msg 1 receive */
	status = ockam_receive_blocking( connection, &recv_buffer[0], MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking for msg 1 failed" );
		goto exit_block;
	}

	/* Msg 1 process */
	status = xx_responder_m1_process( p_h, recv_buffer, bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m1_receive failed" );
		goto exit_block;
	}

	/* Msg 2 make */
	status = xx_responder_m2_make( p_h, send_buffer, sizeof(send_buffer), &transmit_size );
	if(status != OCKAM_ERR_NONE) {
		print_uint8_str( send_buffer, transmit_size, "Sending msg 2:");
		log_error( status, "responder_m2_send failed" );
		goto exit_block;
	}

	/* Msg 2 send */
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m2_send failed" );
		goto exit_block;
	}

	/* Msg 3 receive */
	status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking failed for msg 3" );
		goto exit_block;
	}

	/* Msg 3 process */
	status = xx_responder_m3_process( p_h, recv_buffer,  bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m3_process failed for msg 3" );
		goto exit_block;
	}

	/* Epilogue */
	status = xx_responder_epilogue( p_h );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "Failed responder_epilogue" );
		goto exit_block;
	}

exit_block:
	return status;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

OCKAM_ERR xx_responder_m1_process( XX_HANDSHAKE* p_h, uint8_t* p_m1, uint16_t m1_size )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint16_t        offset = 0;
	uint8_t         key[KEY_SIZE];
	uint32_t        key_bytes;

	// Read 32 bytes from the incoming message buffer
	// parse it as a public key, set it to re
	// h = SHA256(h || re)
	memcpy( p_h->re, p_m1, KEY_SIZE );
	offset += KEY_SIZE;

	mix_hash( p_h, p_h->re, KEY_SIZE );

	// h = SHA256( h || payload )
	mix_hash( p_h, NULL, 0 );

	if( offset != m1_size ) {
		status = OCKAM_ERR_XX_HANDSHAKE_FAILED;
		log_error( status, "handshake failed in  responder_m1_process (size mismatch)");
	}

exit_block:
	return status;
}

OCKAM_ERR xx_responder_m2_make( XX_HANDSHAKE* p_h, uint8_t* p_msg, uint16_t msg_size, uint16_t* p_bytes_written )
{

	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t         cipher_text[MAX_TRANSMIT_SIZE];
	uint16_t        offset = 0;
	uint8_t         vector[VECTOR_SIZE];

	// 1. h = SHA256(h || e.PublicKey),
	// Write e.PublicKey to outgoing message
	// buffer, BigEndian
	mix_hash( p_h, p_h->e, KEY_SIZE );
	memcpy( p_msg, p_h->e, sizeof(p_h->e) );
	offset += sizeof(p_h->e);

	// 2. ck, k = HKDF(ck, DH(e, re), 2)
	// n = 0
	status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL, p_h->re, sizeof(p_h->re), KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh of prologue in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce = 0;

	// 3. c = ENCRYPT(k, n++, h, s.PublicKey)
	// h =  SHA256(h || c),
	// Write c to outgoing message buffer
	memset( cipher_text, 0, sizeof(cipher_text) );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
			p_h->h, sizeof(p_h->h), &cipher_text[KEY_SIZE], TAG_SIZE,  p_h->s, KEY_SIZE, cipher_text, KEY_SIZE);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce += 1;

	mix_hash( p_h, cipher_text, KEY_SIZE+TAG_SIZE );

	// Copy cypher text into send buffer
	memcpy( p_msg+offset, cipher_text, KEY_SIZE+TAG_SIZE );
	offset += KEY_SIZE+TAG_SIZE;

	// 4. ck, k = HKDF(ck, DH(s, re), 2)
	// n = 0
	status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_STATIC, p_h->re, sizeof(p_h->re),
	                  KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce = 0;

	// 5. c = ENCRYPT(k, n++, h, payload)
	// h = SHA256(h || c),
	// payload is empty
	memset( cipher_text, 0, sizeof(cipher_text) );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), &cipher_text[0], TAG_SIZE, NULL, 0, NULL, 0);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce += 1;
	memcpy( p_msg+offset, cipher_text, TAG_SIZE );
	offset += TAG_SIZE;
	mix_hash( p_h, cipher_text, TAG_SIZE );

	// Done
	*p_bytes_written = offset;

exit_block:
	return status;
}

OCKAM_ERR xx_responder_m3_process( XX_HANDSHAKE* p_h, uint8_t* p_m3, uint16_t m3_size )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t         uncipher[MAX_TRANSMIT_SIZE];
	uint8_t         tag[TAG_SIZE];
	uint8_t         vector[VECTOR_SIZE];
	uint32_t        offset = 0;

	// 1. Read 48 bytes the incoming message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a public key,
	// set it to rs
	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_m3+offset+KEY_SIZE, TAG_SIZE );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), tag, sizeof(tag), p_m3, KEY_SIZE, uncipher, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in responder_m3_process");
		goto exit_block;
	}
	memcpy(p_h->rs, uncipher, KEY_SIZE);
	mix_hash( p_h, p_m3+offset, KEY_SIZE+TAG_SIZE );
	offset += KEY_SIZE+TAG_SIZE;

	// 2. ck, k = HKDF(ck, DH(e, rs), 2)
	// n = 0
	status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL,
	                  p_h->rs, sizeof(p_h->rs), KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh in responder_m3_process");
		goto exit_block;
	}
	p_h->nonce = 0;

	// 3. Read remaining bytes of incoming message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a payload,
	// payload should be empty
	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_m3+offset, TAG_SIZE );
	make_vector( p_h->nonce, vector );
	memset(uncipher, 0, sizeof(uncipher));
	status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), tag, sizeof(tag), NULL, 0, NULL, 0 );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
		goto exit_block;
	}
	p_h->nonce += 1;
	mix_hash( p_h, p_m3+offset, TAG_SIZE );
	offset += TAG_SIZE;

exit_block:
	return status;
}

OCKAM_ERR xx_responder_epilogue( XX_HANDSHAKE* p_h )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     keys[2*KEY_SIZE];

	memset(keys, 0, sizeof(keys));
	status = ockam_vault_hkdf( p_h->ck, KEY_SIZE, NULL, 0,  NULL, 0, keys, sizeof(keys));
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_vault_hkdf failed in responder_epilogue_make");
		goto exit_block;
	}
	memcpy(p_h->ke, keys, KEY_SIZE );
	memcpy(p_h->kd, &keys[KEY_SIZE], KEY_SIZE );
	p_h->ne = 0;
	p_h->nd = 0;

exit_block:
	return status;
}
