#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"
#include "handshake.h"

OCKAM_VAULT_CFG_s vault_cfg =
{
		.p_tpm                       = 0,
		.p_host                      = 0,
		OCKAM_VAULT_EC_CURVE25519
};

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
	if( KEY_SIZE != key_bytes ) printf("********oh no*********");
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
	print_uint8_str( p_h->re, KEY_SIZE, "\nM1 re: ");

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
	print_uint8_str( p_h->k, KEY_SIZE, "M2 k1:");

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
	print_uint8_str(p_h->k, KEY_SIZE, "M2 k2:");
	print_uint8_str(p_h->h, SHA256_SIZE, "h");
	p_h->nonce = 0;

	// 5. c = ENCRYPT(k, n++, h, payload)
	// h = SHA256(h || c),
	// payload is empty
	memset( cipher_text, 0, sizeof(cipher_text) );
	make_vector( p_h->nonce, vector );
	print_uint8_str( p_h->k, KEY_SIZE, "M2 encrypt params:\nk: " );
	print_uint8_str( vector, sizeof(vector), "Vector:");
	print_uint8_str( p_h->h, SHA256_SIZE, "h:");
	status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
			p_h->h, sizeof(p_h->h), &cipher_text[payload_size], TAG_SIZE, NULL, 0, NULL, 0);
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
		goto exit_block;
	}
	print_uint8_str(&cipher_text[payload_size], TAG_SIZE, "-----M2 encrypt2 tag:");
	p_h->nonce += 1;
	memcpy( p_msg+offset, cipher_text, payload_size+TAG_SIZE );
	offset += payload_size + TAG_SIZE;
	print_uint8_str(cipher_text, TAG_SIZE+payload_size, "TAG");
	status = mix_hash( p_h, cipher_text, payload_size+TAG_SIZE );

	// Done
	*p_bytes_written = offset;

exit_block:
	return status;
}

OCKAM_ERR responder_m3_process( HANDSHAKE* p_h, uint8_t* p_m3, uint16_t m3_size )
{
	printf("\n\n************M3*************\n");

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
	print_uint8_str( uncipher, 4, "M3 payload");
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
	print_uint8_str( p_h->ke, KEY_SIZE, "--------encrypt key--------");
	print_uint8_str( p_h->kd, KEY_SIZE, "--------decrypt key--------");
	p_h->ne = 0;
	p_h->nd = 0;

exit_block:
	return status;
}

OCKAM_ERR get_ip_info( OCKAM_INTERNET_ADDRESS* p_address )
{

	OCKAM_ERR   status		= OCKAM_ERR_NONE;
	FILE*       address_file;
	char        listen_address[100];
	char        port_str[8];
	unsigned    port = 0;

	// Read the IP address to bind to
	address_file = fopen("../ipaddress.txt", "r");
	if(NULL == address_file) {
		printf("Create a file called \"ipaddress.txt\" with the IP address to listen on," \
			"in nnn.nnn.nnn.nnn format and port number\n");
		status = OCKAM_ERR_INVALID_PARAM;
		goto exit_block;
	}
	fscanf(address_file, "%s\n", &listen_address[0]);
	fscanf(address_file, "%s\n", &port_str[0]);
	port = strtoul( &port_str[0], NULL, 0 );
	fclose(address_file);

	memset( p_address, 0, sizeof( *p_address));

	strcpy( &p_address->ip_address[0], &listen_address[0] );
	p_address->port = port;

exit_block:
	return status;
}

OCKAM_ERR establish_responder_connection( OCKAM_TRANSPORT_CONNECTION* p_listener, OCKAM_TRANSPORT_CONNECTION* p_connection )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	OCKAM_LISTEN_ADDRESS            listener_address;
	OCKAM_TRANSPORT_CONNECTION      connection = NULL;
	OCKAM_TRANSPORT_CONNECTION      listener = NULL;

	// Get the IP address to listen on
	status = get_ip_info( &listener_address.internet_address );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed to get address into");
		goto exit_block;
	}

	status = ockam_init_posix_tcp_connection( &listener );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed ockam_init_posix_tcp_connection");
		goto exit_block;
	}
	// Wait for a connection
	status = ockam_listen_blocking( listener, &listener_address, &connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "listen failed" );
		goto exit_block;
	}

	*p_connection = connection;

exit_block:
	return status;
}

int main() {
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	OCKAM_TRANSPORT_CONNECTION      connection = NULL;
	OCKAM_TRANSPORT_CONNECTION      listener = NULL;
	HANDSHAKE                       handshake;
	uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
	uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
	uint16_t                        transmit_size = 0;
	uint16_t                        bytes_received = 0;
	uint8_t                         epilogue[16];
	uint32_t                        epilogue_size;
	char                            user_msg[80];
	uint8_t*                        p_user_msg = (uint8_t*)user_msg;

	init_err_log(stdout);

	/*-------------------------------------------------------------------------
	 * Establish transport connection with responder
	 *-----------------------------------------------------------------------*/

	status = establish_responder_connection( &listener, &connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "Failed to establish connection with responder");
		goto exit_block;
	}

	status = ockam_vault_init((void*) &vault_cfg);                 /* Initialize vault                                   */
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
	print_uint8_str( (uint8_t*)&recv_buffer[0], bytes_received, "Msg 1:\n");

	/* Msg 1 process */
	status = responder_m1_process( &handshake, recv_buffer, bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m1_receive failed" );
		goto exit_block;
	}

	/* Msg 2 make */
	status = responder_m2_make( &handshake, NULL, 0, send_buffer, sizeof(send_buffer), &transmit_size );
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
	print_uint8_str( send_buffer, transmit_size, "Msg 2 sent: " );

	/* Msg 3 receive */
	status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking failed for msg 3" );
		goto exit_block;
	}
	print_uint8_str( (uint8_t*)&recv_buffer[0], bytes_received, "Msg 3:\n");

	/* Msg 3 process */
	status = responder_m3_process( &handshake, recv_buffer,  bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_m3_process failed for msg 3" );
		goto exit_block;
	}

	/* Epilog make */
	printf("\n---------Epilogue Send----------\n");
	status = responder_epilogue(&handshake);
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "Failed responder_epilogue" );
	}
	string_to_hex(EPI_RESPONDER, epilogue, &epilogue_size );
	print_uint8_str( epilogue, epilogue_size, "Epilogue:");
	status = encrypt( &handshake, epilogue, epilogue_size,
			send_buffer, sizeof(send_buffer), &transmit_size );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_epilogue_make failed" );
		goto exit_block;
	}
	printf("\n");

	/* Epilogue send */
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_send_blocking epilogue failed" );
		goto exit_block;
	}
	print_uint8_str( send_buffer, transmit_size, "Epilogue sent: " );

	/* Epilogue receive */
	status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking failed for msg 3" );
		goto exit_block;
	}
	print_uint8_str( (uint8_t*)&recv_buffer[0], bytes_received, "Msg 3:\n");

	// Epilogue process
	status = decrypt( &handshake, epilogue, EPI_BYTE_SIZE, recv_buffer, bytes_received, &epilogue_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}
	print_uint8_str( epilogue, EPI_BYTE_SIZE, "-------Epilogue received---------");

	/* Epi-epilogue */
	printf("Enter a string to encrypt and send: ");
	getline( (char**)&p_user_msg, (size_t*)&transmit_size, stdin );
	status = encrypt( &handshake, p_user_msg, strlen((char*)p_user_msg)+1, send_buffer, sizeof(send_buffer), &transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "encrypt failed on user message" );
		goto exit_block;
	}
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_send_blocking failed on user message" );
		goto exit_block;
	}
	print_uint8_str( send_buffer, transmit_size, "Encrypted: ");
	printf("Type anything to quit\n");
	scanf("%s", p_user_msg );

exit_block:
	if( NULL != connection ) ockam_uninit_connection( connection );
	if( NULL != listener ) ockam_uninit_connection( listener );
	return status;
}
