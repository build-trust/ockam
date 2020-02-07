#include <stdio.h>
#include "ockam/syslog.h"
#include "ockam/transport.h"

char* p_file_to_send =  "./test_data_server.txt";
char* p_file_to_receive = "./test_data_client.txt";
char* p_file_to_compare = "./test_data_compare.txt";


OCKAM_ERR file_compare( char* p_f1, char* p_f2 )
{
	OCKAM_ERR    status = 0;

	uint16_t    more = 1;

	FILE*       fp1 = NULL;
	FILE*       fp2 = NULL;

	char        buffer1[256];
	char        buffer2[256];

	size_t      r1;
	size_t      r2;

	fp1 = fopen( p_f1, "r" );
	fp2 = fopen( p_f2, "r" );

	if( (NULL == fp1) || (NULL == fp2)) {
		status = OCKAM_ERR_TRANSPORT_TEST;
		goto exit_block;
	}

	while( more ) {
		r1 = fread( buffer1, 1, sizeof( buffer1 ), fp1 );
		r2 = fread( buffer2, 1, sizeof( buffer2 ), fp2 );
		if( r1 != r2 ) {
			status = OCKAM_ERR_TRANSPORT_TEST;
			goto exit_block;
		}
		if( 0 != memcmp( buffer1, buffer2, r1 )) {
			status = OCKAM_ERR_TRANSPORT_TEST;
			goto exit_block;
		}
		if( feof(fp1) ) {
			if(!feof(fp2)) {
				status = OCKAM_ERR_TRANSPORT_TEST;
				goto exit_block;
			}
			more = 0;
		}
	}

exit_block:
	if( NULL != fp1 ) fclose( fp1 );
	if( NULL != fp2 ) fclose( fp2 );
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
	address_file = fopen("ipaddress.txt", "r");
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

int16_t listen_callback( void* context, OCKAM_TRANSPORT_CONNECTION connection );

int main() {

	OCKAM_ERR                       status              = OCKAM_ERR_NONE;
	OCKAM_TRANSPORT_CONNECTION      connection          = NULL;
	OCKAM_TRANSPORT_CONNECTION      listener            = NULL;
	OCKAM_LISTEN_ADDRESS            listen_address;
	char                            send_buffer[64];
	uint16_t                        send_length;
	char                            receive_buffer[64];
	uint16_t                        bytes_received      = 0;
	FILE*                           file_send           = NULL;
	FILE*                           file_receive        = NULL;
	size_t                          bytes_written;
	uint16_t                        send_not_done       = 1;
	uint16_t                        receive_not_done    = 1;

	init_err_log( stdin );

	// Initialize TCP connection
	status = ockam_init_posix_tcp_connection( &listener );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed ockam_init_posix_tcp_connection");
		goto exit_block;
	}

	status = get_ip_info( &listen_address.internet_address );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed to get address info");
		goto exit_block;
	}

	// Open the test data file for sending
	file_send = fopen( p_file_to_send, "r" );
	if( NULL == file_send ) {
		status = OCKAM_ERR_TRANSPORT_TEST;
		log_error( status, "failed to open test file test_data_client.txt");
		goto exit_block;
	}

	// Create file for test data received
	file_receive = fopen( p_file_to_receive, "w" );
	if( NULL == file_receive ) {
		status = OCKAM_ERR_TRANSPORT_TEST;
		log_error( status, "failed to open test file test_data_client.txt");
		goto exit_block;
	}


	// Listen (blocking) for a connection
	status = ockam_listen_blocking( listener, &listen_address, &connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "listen failed" );
		goto exit_block;
	}

	while( receive_not_done ) {
		status = ockam_receive_blocking( connection, &receive_buffer[0], sizeof(receive_buffer), &bytes_received );
		if ( OCKAM_ERR_NONE != status ) {
			log_error( status, "Receive failed" );
			goto exit_block;
		}
		// Look for special "the end" buffer
		if ( 0 == strncmp( "that's all", &receive_buffer[0], strlen( "that's all" ))) {
			receive_not_done = 0;
		} else {
			bytes_written = fwrite( &receive_buffer[0], 1, bytes_received, file_receive );
			if ( bytes_written != bytes_received ) {
				log_error( OCKAM_ERR_TRANSPORT_TEST, "failed write to output file" );
				goto exit_block;
			}
		}
	}
	fclose( file_receive );

	// Send the test data file
	while( send_not_done ) {
		send_length = fread( &send_buffer[0], 1, sizeof( send_buffer ), file_send );
		if ( feof( file_send )) send_not_done = 0;
		status = ockam_send_blocking( connection, &send_buffer[0], send_length );
		if ( OCKAM_ERR_NONE != status ) {
			log_error( status, "Send failed" );
			goto exit_block;
		}
	}
	// Send special "the end" buffer
	status = ockam_send_blocking( connection, "that's all", strlen("that's all")+1 );
	if ( OCKAM_ERR_NONE != status ) {
		log_error( status, "Send failed" );
		goto exit_block;
	}

	fclose( file_send );

	// Now compare the received file and the reference file
	if( 0 != file_compare( p_file_to_receive, p_file_to_compare )) {
		status = OCKAM_ERR_TRANSPORT_TEST;
		log_error( status, "file compare failed" );
		goto exit_block;
	}

exit_block:
	if( NULL != connection ) ockam_uninit_connection( connection );
	if( NULL != listener ) ockam_uninit_connection( listener );
	return status;
}

