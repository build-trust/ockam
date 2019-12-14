#include <stdio.h>

#include "transport.h"
#include "error.h"
#include "errlog.h"

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

int main(int argc, char* argv[]) {
	OCKAM_TRANSPORT		        transport = NULL;
	OCKAM_ERR					error = 0;
	OCKAM_INTERNET_ADDRESS		address;
	char						buffer[128];
	unsigned int                bytes_received = 0;

	init_err_log(stdout);

	// Get IP address and port
	error = get_ip_info( &address );
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_get_device_record");
		goto exit_block;
	}

	error = ockam_init_posix_socket_tcp_server(&address, &transport);
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_xp_init_IP_CONNECTION");
		goto exit_block;
	}

	error = ockam_receive(transport, &buffer[0], sizeof(buffer), &bytes_received);
	if (OCKAM_ERR_NONE != error) {
		log_error("failed ockam_xp_receive");
		goto exit_block;
	}

	printf("%d Bytes, %s\n", bytes_received, buffer);

exit_block:
	if( NULL != transport ) ockam_uninit_transport( transport );
	return 0;
}
