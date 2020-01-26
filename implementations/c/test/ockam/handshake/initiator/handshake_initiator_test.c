#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"
#include "ockam/handshake.h"
#include "handshake_test.h"

OCKAM_VAULT_CFG_s vault_cfg =
{
		.p_tpm                       = 0,
		.p_host                      = 0,
		OCKAM_VAULT_EC_CURVE25519
};

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

OCKAM_ERR establish_connection( OCKAM_TRANSPORT_CONNECTION* p_connection )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	OCKAM_INTERNET_ADDRESS          responder_address;
	OCKAM_TRANSPORT_CONNECTION      connection = NULL;

	// Get the IP address of the responder
	status = get_ip_info( &responder_address );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed to get address into");
		goto exit_block;
	}

	status = ockam_init_posix_tcp_connection( &connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed ockam_init_posix_tcp_connection");
		goto exit_block;
	}
	// Try to connect
	status = ockam_connect_blocking( &responder_address, connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "connect failed" );
		goto exit_block;
	}

	*p_connection = connection;

exit_block:
	return status;
}

int main() {
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	OCKAM_TRANSPORT_CONNECTION      connection;
	HANDSHAKE                       handshake;
	uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
	uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
	uint16_t                        bytes_received = 0;
	uint16_t                        transmit_size = 0;
	uint8_t                         epi[EPI_BYTE_SIZE];
	uint32_t                        epi_size;
	uint32_t                        epi_bytes;
	char                            user_msg[80];
	uint8_t*                        p_user_msg = (uint8_t*)user_msg;
	uint32_t                        user_bytes;

	init_err_log(stdout);

	/*-------------------------------------------------------------------------
	 * Establish transport connection with responder
	 *-----------------------------------------------------------------------*/

	status = establish_connection( &connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error(status, "Failed to establish connection with responder");
		goto exit_block;
	}

	status = ockam_vault_init((void*) &vault_cfg);                 /* Initialize vault                                   */
	if(status != OCKAM_ERR_NONE) {
		log_error( status, "ockam_vault_init failed" );
		goto exit_block;
	}

	// Step 1 generate message
	status = initiator_m1_make( &handshake,  NULL, 0, NULL, 0, send_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
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
	status = initiator_m2_process( &handshake, recv_buffer, bytes_received );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}

	// Msg 3 make
	status = initiator_m3_make( &handshake, send_buffer, &transmit_size );
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

	// Epilogue
	status = initiator_epilogue( &handshake );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "initiator_epilogue failed" );
		goto exit_block;
	}

	// Epilogue receive
	status = ockam_receive_blocking( connection, recv_buffer, sizeof(recv_buffer), &bytes_received );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}

	// Epilogue process
	status = decrypt( &handshake, epi, EPI_BYTE_SIZE, recv_buffer, bytes_received,  &epi_bytes );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on msg 2" );
		goto exit_block;
	}
	print_uint8_str( epi, EPI_BYTE_SIZE, "-------Epilogue received---------");

	// Epilogue make
	string_to_hex(EPI_INITIATOR, epi, &epi_size );
	print_uint8_str(epi, epi_size, "hex epilogue");
	status = encrypt( &handshake, epi, epi_size, send_buffer, sizeof(send_buffer), &transmit_size) ;
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "initiator_encrypt failed on epilogue" );
		goto exit_block;
	}

	// Epilogue send
	status = ockam_send_blocking( connection, send_buffer, transmit_size );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_send_blocking failed on msg 3" );
		goto exit_block;
	}

	/* Get user message */
	status = ockam_receive_blocking( connection, recv_buffer, sizeof(recv_buffer), &bytes_received );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "ockam_receive_blocking failed on user message" );
		goto exit_block;
	}
	print_uint8_str(recv_buffer, bytes_received, "Encrypted: ");
	printf("----\n");
	status = decrypt( &handshake, p_user_msg, 80, recv_buffer, bytes_received, &user_bytes );
	print_uint8_str( p_user_msg, user_bytes, "Decrypted message: ");
	printf("%s\n", p_user_msg);

exit_block:
	printf( "Test ended with status 0x%4x", status );
	return status;
}
