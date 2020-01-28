#include <string.h>
#include "ockam/vault.h"
#include "ockam/error.h"
#include "ockam/syslog.h"
#include "ockam/handshake.h"


OCKAM_VAULT_CFG_s vault_cfg =
		{
				.p_tpm                       = 0,
				.p_host                      = 0,
				OCKAM_VAULT_EC_CURVE25519
		};


OCKAM_ERR mix_hash( HANDSHAKE* p_handshake,  uint8_t* p_bytes, uint16_t b_length );
OCKAM_ERR make_vector( uint64_t nonce, uint8_t* p_vector );
OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 );

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */
OCKAM_ERR ockam_initiator_handshake( OCKAM_TRANSPORT_CONNECTION connection, HANDSHAKE* p_h )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
	uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
	uint16_t                        bytes_received = 0;
	uint16_t                        transmit_size = 0;


	status = ockam_vault_init((void*) &vault_cfg);                 /* Initialize vault                                   */
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_vault_init failed" );
		goto exit_block;
	}

	// Step 1 generate message
	status = initiator_m1_make( p_h,  NULL, 0, NULL, 0, send_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "initiator_step_1 failed" );
		goto exit_block;
	}

	// Step 1 send message
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_send_blocking after initiator_step_1 failed" );
		goto exit_block;
	}

	// Msg 2 receive
	status = ockam_receive_blocking( connection, recv_buffer, sizeof(recv_buffer), &bytes_received );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}

	// Msg 2 process
	status = initiator_m2_process( p_h, recv_buffer, bytes_received );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}

	// Msg 3 make
	status = initiator_m3_make( p_h, send_buffer, &transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "initiator_m3_make failed" );
		goto exit_block;
	}
	// Msg 3 send
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_send_blocking failed on msg 3" );
		goto exit_block;
	}

	status = initiator_epilogue( p_h );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "initiator_epilogue failed" );
		goto exit_block;
	}

exit_block:
	return status;
}

OCKAM_ERR ockam_responder_handshake( OCKAM_TRANSPORT_CONNECTION connection, HANDSHAKE* p_h )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
	uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
	uint16_t                        transmit_size = 0;
	uint16_t                        bytes_received = 0;

	/* Initialize vault                                   */
	status = ockam_vault_init((void*) &vault_cfg);
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_vault_init failed" );
		goto exit_block;
	}

	/* Msg 1 receive */
	status = ockam_receive_blocking( connection, &recv_buffer[0], MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking for msg 1 failed" );
		goto exit_block;
	}

	/* Msg 1 process */
	status = responder_m1_process( p_h, recv_buffer, bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m1_receive failed" );
		goto exit_block;
	}

	/* Msg 2 make */
	status = responder_m2_make( p_h, NULL, 0, send_buffer, sizeof(send_buffer), &transmit_size );
	if(status != OCKAM_ERR_NONE) {
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
	status = responder_m3_process( p_h, recv_buffer,  bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m3_process failed for msg 3" );
		goto exit_block;
	}

	/* Epilogue - make shared secret */
	status = responder_epilogue( p_h );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "Failed responder_epilogue" );
		goto exit_block;
	}

exit_block:
	return status;
}

OCKAM_ERR encrypt( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                   uint8_t* p_msg, uint16_t msg_length, uint16_t* p_msg_size )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     cipher_text[MAX_TRANSMIT_SIZE];
	uint8_t     vector[VECTOR_SIZE];
	uint32_t    offset = 0;

	if( msg_length < (payload_size+TAG_SIZE)) {
		status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
		goto exit_block;
	}

	memset( cipher_text, 0, sizeof(cipher_text) );
	make_vector( p_h->ne, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->ke, KEY_SIZE, vector, sizeof(vector),
	                                      NULL, 0, &cipher_text[payload_size], TAG_SIZE, p_payload,
	                                      payload_size, cipher_text, payload_size);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in encrypt");
		goto exit_block;
	}
	memcpy( p_msg, cipher_text, TAG_SIZE+payload_size );
	offset += TAG_SIZE+payload_size;
	p_h->ne += 1;
	*p_msg_size = offset;

exit_block:
	return status;
}


OCKAM_ERR decrypt( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                   uint8_t* p_msg, uint16_t msg_length, uint32_t* p_payload_bytes )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     uncipher[MAX_TRANSMIT_SIZE];
	uint8_t     tag[TAG_SIZE];
	uint8_t     vector[VECTOR_SIZE];
	uint32_t    offset = 0;
	uint32_t    uncipher_size = 0;

	if( payload_size < (msg_length-TAG_SIZE)) {
		status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
		goto exit_block;
	}

	*p_payload_bytes = msg_length-TAG_SIZE;

	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_msg+offset+*p_payload_bytes, TAG_SIZE );
	make_vector( p_h->nd, vector );
	memset(uncipher, 0, sizeof(uncipher));
	uncipher_size = msg_length-TAG_SIZE;
	status = ockam_vault_aes_gcm_decrypt( p_h->kd, KEY_SIZE, vector, sizeof(vector),
	                                      NULL, 0, tag, sizeof(tag), p_msg+offset, uncipher_size, uncipher, uncipher_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
		goto exit_block;
	}
	memcpy( p_payload, uncipher, payload_size );
	p_h->nd += 1;

exit_block:
	return status;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*------------------------------------------------------------------------------------------------------*
 *          RESPONDER FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/
OCKAM_ERR responder_m1_process( HANDSHAKE* p_h, uint8_t* p_m1, uint16_t m1_size )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint16_t        m1_offset = 0;
	uint8_t         key[KEY_SIZE];
	uint32_t        key_bytes;

	// 1. Pick a static 25519 keypair for this handshake and set it to s
	string_to_hex( RESPONDER_STATIC, key, &key_bytes );
	if( KEY_SIZE != key_bytes ) printf("********oh no*********");
	status = ockam_vault_key_write( OCKAM_VAULT_KEY_STATIC, key, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate static keypair in initiator_step_1");
		goto exit_block;
	}

	status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_STATIC, p_h->s, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate get static public key in initiator_step_1");
		goto exit_block;
	}

	// 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
	string_to_hex( RESPONDER_EPH, key, &key_bytes );
	status = ockam_vault_key_write( OCKAM_VAULT_KEY_EPHEMERAL, key, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate static keypair in initiator_step_1");
		goto exit_block;
	}

	status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_EPHEMERAL, p_h->e, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate get static public key in initiator_step_1");
		goto exit_block;
	}

	// 3. Set k to empty, Set n to 0
	p_h->nonce = 0;
	memset( p_h->k, 0, KEY_SIZE );

	// 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
	memset( p_h->h, 0, SHA256_SIZE );
	memcpy( p_h->h, NAME, NAME_SIZE );
	memset( p_h->ck, 0, SHA256_SIZE );
	memcpy( p_h->ck, NAME, NAME_SIZE );

	// 5. h = SHA256(h || prologue),
	// prologue is empty
	mix_hash( p_h, NULL, 0 );

	// 6. Read 32 bytes from the incoming message buffer
	// parse it as a public key, set it to re
	// h = SHA256(h || re)
	memcpy( p_h->re, p_m1, KEY_SIZE );
	m1_offset += KEY_SIZE;

	mix_hash( p_h, p_h->re, KEY_SIZE );

	// h = SHA256( h || payload )
	status = mix_hash( p_h, NULL, 0 );

exit_block:
	return status;
}

OCKAM_ERR responder_m2_make( HANDSHAKE* p_h, uint8_t* p_payload, uint32_t payload_size,
                             uint8_t* p_msg, uint16_t msg_size, uint16_t* p_bytes_written )
{

	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t         cipher_text[MAX_TRANSMIT_SIZE];
	uint16_t        offset = 0;
	uint8_t         vector[VECTOR_SIZE];

	// Make sure msg is big enough
	if( msg_size < payload_size+TAG_SIZE ) {
		status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
		goto exit_block;
	}

	// 1. h = SHA256(h || e.PublicKey),
	// Write e.PublicKey to outgoing message
	// buffer, BigEndian
	status = mix_hash( p_h, p_h->e, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash of prologue in initiator_step_1");
		goto exit_block;
	}
	memcpy( p_msg, p_h->e, sizeof(p_h->e) );
	offset += sizeof(p_h->e);

	// 2. ck, k = HKDF(ck, DH(e, re), 2)
	// n = 0
	// secret = ECDH( e, re )
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

	status = mix_hash( p_h, cipher_text, KEY_SIZE+TAG_SIZE );

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
	                                      p_h->h, sizeof(p_h->h), &cipher_text[payload_size], TAG_SIZE, NULL, 0, NULL, 0);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce += 1;
	memcpy( p_msg+offset, cipher_text, payload_size+TAG_SIZE );
	offset += payload_size + TAG_SIZE;
	status = mix_hash( p_h, cipher_text, payload_size+TAG_SIZE );

	// Done
	*p_bytes_written = offset;

exit_block:
	return status;
}

OCKAM_ERR responder_m3_process( HANDSHAKE* p_h, uint8_t* p_m3, uint16_t m3_size )
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

OCKAM_ERR responder_epilogue( HANDSHAKE* p_h )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     keys[2*KEY_SIZE];

	memset(keys, 0, sizeof(keys));
	status = ockam_vault_hkdf( NULL, 0, p_h->ck, KEY_SIZE, NULL, 0, keys, sizeof(keys));
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

/*------------------------------------------------------------------------------------------------------*
 *          INITIATOR FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/

OCKAM_ERR initiator_m1_make( HANDSHAKE* p_h, uint8_t* p_prologue, uint16_t prologue_length,
                             uint8_t* p_payload, uint16_t payload_length, uint8_t* p_send_buffer, uint16_t buffer_length,
                             uint16_t* p_transmit_size )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint16_t        buffer_idx = 0;
	uint16_t        transmit_size = 0;
	uint8_t         key[KEY_SIZE];
	uint32_t        key_bytes;

	// 1. Pick a static 25519 keypair for this handshake and set it to s
	string_to_hex( INITIATOR_STATIC, key, &key_bytes );
	status = ockam_vault_key_write( OCKAM_VAULT_KEY_STATIC, key, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate static keypair in initiator_step_1");
		goto exit_block;
	}

	status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_STATIC, p_h->s, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate get static public key in initiator_step_1");
		goto exit_block;
	}

	// 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
	string_to_hex( INITIATOR_EPH, key, &key_bytes );
	status = ockam_vault_key_write( OCKAM_VAULT_KEY_EPHEMERAL, key, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate static keypair in initiator_step_1");
		goto exit_block;
	}

	status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_EPHEMERAL, p_h->e, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed to generate get static public key in initiator_step_1");
		goto exit_block;
	}

	// Nonce to 0, k to empty
	p_h->nonce = 0;
	memset(p_h->k, 0, sizeof(p_h->k));

	// Initialize h to "Noise_XX_25519_AESGCM_SHA256" and set prologue to empty
	memset( &p_h->h[0], 0, SHA256_SIZE );
	memcpy( &p_h->h[0], NAME, NAME_SIZE );

	// Initialize ck
	memset( &p_h->ck[0], 0, SHA256_SIZE );
	memcpy( &p_h->ck[0], NAME, NAME_SIZE );

	// h = SHA256(h || prologue), prologue is empty
	status = mix_hash( p_h, p_prologue, prologue_length );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash of prologue in initiator_step_1");
		goto exit_block;
	}

	// Write e to outgoing buffer
	// h = SHA256(h || e.PublicKey
	memcpy( p_send_buffer, p_h->e, KEY_SIZE );
	transmit_size += KEY_SIZE;

	status = mix_hash( p_h, p_h->e, sizeof(p_h->e) );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash of e in initiator_step_1");
		goto exit_block;
	}

	// Write payload to outgoing buffer, payload is empty
	// h = SHA256( h || payload )
	memcpy( p_send_buffer, p_payload, payload_length );
	transmit_size += payload_length;

	status = mix_hash( p_h, p_payload, payload_length );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash of payload in initiator_step_1");
		goto exit_block;
	}

	*p_transmit_size = transmit_size;

exit_block:
	return status;
}

OCKAM_ERR initiator_m2_process( HANDSHAKE* p_h, uint8_t* p_recv, uint16_t recv_size )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint16_t        offset = 0;
	uint8_t         uncipher[MAX_TRANSMIT_SIZE];
	uint8_t         tag[TAG_SIZE];
	uint8_t         vector[VECTOR_SIZE];

	// 1. Read 32 bytes from the incoming
	// message buffer, parse it as a public
	// key, set it to re
	// h = SHA256(h || re)
	memcpy( p_h->re, p_recv, KEY_SIZE );
	offset += KEY_SIZE;
	status = mix_hash( p_h, p_h->re, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash of re in initiator_m2_receive");
		goto exit_block;
	}

	// 2. ck, k = HKDF(ck, DH(e, re), 2)
	// n = 0
	status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL, p_h->re, sizeof(p_h->re),
	                  KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh of prologue in responder_m2_make");
		goto exit_block;
	}
	p_h->nonce = 0;

	// 3. Read 48 bytes of the incoming message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a public key,
	// set it to rs
	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_recv+offset+KEY_SIZE, TAG_SIZE );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), tag, sizeof(tag), p_recv+offset, KEY_SIZE, uncipher, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
		goto exit_block;
	}
	p_h->nonce += 1;
	memcpy( p_h->rs, uncipher, KEY_SIZE );
	status = mix_hash( p_h, p_recv+offset, KEY_SIZE+TAG_SIZE );
	offset += KEY_SIZE + TAG_SIZE;

	// 4. ck, k = HKDF(ck, DH(e, rs), 2)
	// n = 0
	// secret = ECDH( e, re )
	status = hkdf_dh( p_h->ck,  sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL, p_h->rs, sizeof(p_h->rs),
	                  KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh of prologue in initiator_m2_process");
		goto exit_block;
	}
	p_h->nonce = 0;

	// 5. Read remaining bytes of incoming
	// message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a payload,
	// payload should be empty
	memset( tag, 0, sizeof(tag) );
	memcpy( tag, p_recv+offset, TAG_SIZE );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), tag, sizeof(tag), NULL, 0, NULL, 0 );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
		goto exit_block;
	}
	p_h->nonce += 1;
	mix_hash( p_h, p_recv+offset, TAG_SIZE );

exit_block:
	return status;
}

OCKAM_ERR initiator_m3_make( HANDSHAKE* p_h, uint8_t* p_msg, uint16_t* p_msg_size )
{

	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t         tag[TAG_SIZE];
	uint8_t         cipher[KEY_SIZE];
	u_int16_t       offset = 0;
	uint8_t         vector[VECTOR_SIZE];

	// 1. c = ENCRYPT(k, n++, h, s.PublicKey)
	// h =  SHA256(h || c),
	// Write c to outgoing message
	// buffer, BigEndian
	memset( cipher, 0, sizeof(cipher) );
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector), p_h->h, SHA256_SIZE,
	                                      tag, TAG_SIZE, p_h->s, KEY_SIZE, cipher, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in initiator_m3_make");
		goto exit_block;
	}
	p_h->nonce += 1;
	memcpy( p_msg, cipher, KEY_SIZE );
	offset += KEY_SIZE;
	memcpy( p_msg+offset, tag, TAG_SIZE );
	offset += TAG_SIZE;
	status = mix_hash( p_h, p_msg, KEY_SIZE+TAG_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed mix_hash in initiator_m3_make");
		goto exit_block;
	}

	// 2. ck, k = HKDF(ck, DH(s, re), 2)
	// n = 0
	status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_STATIC, p_h->re, sizeof(p_h->re),
	                  KEY_SIZE, p_h->ck, p_h->k );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed hkdf_dh in initiator_m3_make");
		goto exit_block;
	}
	p_h->nonce = 0;
	print_uint8_str(p_h->k, KEY_SIZE, "M3 k1");

	// 3. c = ENCRYPT(k, n++, h, payload)
	// h = SHA256(h || c),
	// payload is empty
	make_vector( p_h->nonce, vector );
	status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
	                                      p_h->h, sizeof(p_h->h), cipher, TAG_SIZE, NULL, 0, NULL, 0 );
	p_h->nonce += 1;
	mix_hash(p_h, cipher, TAG_SIZE);
	memcpy( p_msg+offset, cipher, TAG_SIZE );
	offset += TAG_SIZE;
	// Copy cipher text into send buffer, append tag

	*p_msg_size = offset;

exit_block:
	return status;
}


OCKAM_ERR initiator_epilogue( HANDSHAKE* p_h )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	uint8_t     keys[2*KEY_SIZE];

	memset(keys, 0, sizeof(keys));
	status = ockam_vault_hkdf( NULL, 0, p_h->ck, KEY_SIZE, NULL, 0, keys, sizeof(keys));
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_vault_hkdf failed in responder_epilogue_make");
	}
	memcpy(p_h->kd, keys, KEY_SIZE );
	memcpy(p_h->ke, &keys[KEY_SIZE], KEY_SIZE );
	p_h->ne = 0;
	p_h->nd = 0;

exit_block:
	return status;
}

/*------------------------------------------------------------------------------------------------------*
 *          UTILITY FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/
void print_uint8_str( uint8_t* p, uint16_t size, char* msg )
{
	printf("\n%s %d bytes: \n", msg, size);
	for( int i = 0; i < size; ++i ) printf( "%0.2x", *p++ );
	printf("\n");
}


OCKAM_ERR hkdf_dh( uint8_t* hkdf1, uint16_t hkdf1_size, OCKAM_VAULT_KEY_e dh_key, uint8_t*  dh2, uint16_t dh2_size,
                   uint16_t out_size, uint8_t*  out_1, uint8_t*  out_2 )
{
	OCKAM_ERR   status = OCKAM_ERR_NONE;
	uint8_t     pms[KEY_SIZE];
	uint8_t     bytes[2*out_size];

	status = ockam_vault_ecdh( dh_key, dh2, dh2_size, pms, KEY_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_ecdh in responder_m2_send");
		goto exit_block;
	}

	// ck, k = HKDF( ck, pms )
	status = ockam_vault_hkdf( pms, KEY_SIZE, hkdf1, hkdf1_size, NULL, 0, bytes, sizeof(bytes));
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_hkdf in responder_m2_send");
		goto exit_block;
	}
	memcpy( out_1, bytes, out_size );
	memcpy( out_2, bytes, out_size );

exit_block:
	return status;
}

void string_to_hex(char* hexstring, uint8_t* val, uint32_t* p_bytes )
{
	const char* pos = hexstring;
	uint32_t bytes = 0;

	for (size_t count = 0; count < (strlen(hexstring)/2); count++) {
		sscanf(pos, "%2hhx", &val[count]);
		pos += 2;
		bytes += 1;
	}
	if( NULL != p_bytes ) *p_bytes = bytes;
}

OCKAM_ERR mix_hash( HANDSHAKE* p_handshake,  uint8_t* p_bytes, uint16_t b_length )
{
	OCKAM_ERR       status = OCKAM_ERR_NONE;
	uint8_t*        p_h = &p_handshake->h[0];
	uint8_t         string[MAX_TRANSMIT_SIZE];
	uint8_t         hash[SHA256_SIZE];

	memset( &hash[0], 0, sizeof(hash) );
	memset( &string[0], 0, sizeof(string) );
	memcpy( &string[0], &p_h[0], SHA256_SIZE );
	memcpy( &string[SHA256_SIZE], p_bytes, b_length );
	status = ockam_vault_sha256( (uint8_t *)&string[0], SHA256_SIZE+b_length, (uint8_t *)&hash[0], SHA256_SIZE );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_sha256 in mix_hash");
		goto exit_block;
	}
	memcpy( p_h, hash, SHA256_SIZE );

exit_block:
	return status;
}

OCKAM_ERR make_vector( uint64_t nonce, uint8_t* p_vector )
{
	uint8_t*    pv;
	uint8_t*    pn = (uint8_t*)&nonce;

	memset( p_vector, 0, VECTOR_SIZE );
	pv = p_vector+4;
	pn += 7;
	for( int i = 7; i >= 0; --i ) {
		*pv++ = *pn--;
	}
	return OCKAM_ERR_NONE;
}
