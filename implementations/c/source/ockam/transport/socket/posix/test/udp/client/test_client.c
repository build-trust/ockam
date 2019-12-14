#include <stdio.h>
#include <string.h>

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
		OCKAM_TRANSPORT		    transport = NULL;
	OCKAM_ERR					error = 0;
	OCKAM_INTERNET_ADDRESS		address;
	char                        buffer[80];
	char*                       p_buffer = &buffer[0];
	unsigned long               buffer_size;
	unsigned int                bytes_sent = 0;

	init_err_log(stdout);

	error = get_ip_info( &address);
    if( OCKAM_ERR_NONE != error ) {
        log_error("failed ockam_get_device_record");
        goto exit_block;
    }

	error = ockam_init_posix_socket_udp_client( &address, &transport );
	if(OCKAM_ERR_NONE != error) {
		log_error("ockam_xp_init_client failed");
		goto exit_block;
	}

	do {
		printf("What to send? ");
		p_buffer = &buffer[0];
		buffer_size = 80;
		getline(&p_buffer, &buffer_size, stdin);
		buffer_size = strlen(p_buffer)+1;
		printf("sending %s\n", p_buffer);
		error = ockam_send(transport, (void *) p_buffer, buffer_size, &bytes_sent);
		if (OCKAM_ERR_NONE != error) {
			log_error("ockam_xp_send failed");
			goto exit_block;
		}
	} while('q' != buffer[0]);

exit_block:
	if(NULL != transport) {
		ockam_uninit_transport(transport);
	}
	return 0;
}
