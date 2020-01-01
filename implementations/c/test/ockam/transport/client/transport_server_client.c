#include <stdio.h>
#include "syslog.h"
#include "transport.h"
#include "queue.h"

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

int main() {

	OCKAM_ERR                       status              = OCKAM_ERR_NONE;
	OCKAM_TRANSPORT_CONNECTION      client_connection   = NULL;
	OCKAM_LISTEN_ADDRESS            host_address;

	init_err_log( stdin );

	// Initialize TCP connection
	status = ockam_init_posix_tcp_connection( &client_connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed ockam_init_posix_tcp_connection");
		goto exit_block;
	}

	status = get_ip_info( &host_address.internet_address );

	// Try to connect
	status = ockam_connect_blocking( &host_address, client_connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "connect failed" );
		goto exit_block;
	}

	printf("\nConnected!\n");

	ockam_uninit_connection( client_connection );

exit_block:
	if( NULL != client_connection ) ockam_uninit_connection( client_connection );
	return status;
}
