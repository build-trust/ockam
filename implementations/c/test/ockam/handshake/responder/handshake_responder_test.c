#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"
#include "ockam/handshake.h"
#include "handshake_test.h"

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
	uint8_t                         test[16];
	uint32_t                        test_size;
	uint8_t                         test_initiator[TEST_MSG_BYTE_SIZE];
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

	status = ockam_responder_handshake( connection, &handshake );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_responder_handshake failed" );
		goto exit_block;
	}

	/* Test message make */
	string_to_hex(TEST_MSG_RESPONDER, test, &test_size );
	status = encrypt( &handshake, test, test_size,
			send_buffer, sizeof(send_buffer), &transmit_size );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "responder_epilogue_make failed" );
		goto exit_block;
	}

	/* Test message send */
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_send_blocking epilogue failed" );
		goto exit_block;
	}

	/* Test message receive */
	status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &bytes_received );
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_receive_blocking failed for msg 3" );
		goto exit_block;
	}

	// Test message process
	status = decrypt( &handshake, test, TEST_MSG_BYTE_SIZE, recv_buffer, bytes_received, &test_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}
	string_to_hex( TEST_MSG_INITIATOR, test_initiator, NULL );
	if( 0 != memcmp( (void*)test, test_initiator, TEST_MSG_BYTE_SIZE) ){
		print_uint8_str( test, TEST_MSG_BYTE_SIZE, "Test message decrypted: ");
		status = OCKAM_ERR_HANDSHAKE_FAILED;
		log_error( status, "Received bad test message" );
		goto exit_block;
	}

exit_block:
	if( NULL != connection ) ockam_uninit_connection( connection );
	if( NULL != listener ) ockam_uninit_connection( listener );
	printf( "Test ended with status %0.4x\n", status );
	return status;
}
