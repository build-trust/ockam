#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"
#include "ockam/handshake.h"

OCKAM_VAULT_CFG_s vault_cfg =
{
		.p_tpm                       = 0,
		.p_host                      = 0,
		OCKAM_VAULT_EC_CURVE25519
};

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
	address_file = fopen("../config/ipaddress.txt", "r");
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
