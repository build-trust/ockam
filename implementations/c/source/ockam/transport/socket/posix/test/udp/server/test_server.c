#include <stdio.h>

#include "transport.h"
#include "error.h"
#include "errlog.h"

/**
 * ockam_get_device_record - stub for getting IP addresses & ports
 * @param id
 * @param p_ockam_device
 * @return
 */
OCKAM_ERR ockam_get_device_record( OCKAM_DEVICE_RECORD* p_ockam_device )
{
	OCKAM_ERR   status		= OCKAM_ERR_NONE;
    FILE*       address_file;
    char        listen_address[100];
    char        port_str[8];
    unsigned    port = 0;

    // Read the IP address to bind to
    address_file = fopen("ipaddress.txt", "r");
    if(NULL == address_file) {
        printf("Create a file called \"ipaddress.txt\" containing the IP address to listen on, in nnn.nnn.nnn.nnn format\n");
        status = OCKAM_ERR_INVALID_PARAM;
        goto exit_block;
    }
	fscanf(address_file, "%s\n", &listen_address[0]);
	fscanf(address_file, "%[^\n]", &port_str[0]);
    port = strtoul( &port_str[0], NULL, 0 );
	fclose(address_file);

    memset( p_ockam_device, 0, sizeof( *p_ockam_device));

    strcpy( &p_ockam_device->host_address.ip_address[0], &listen_address[0] );
    p_ockam_device->host_port = port;

    exit_block:
    return status;
}

/**
 * Interactive test program for UDP sender. Reads a line of input from stdin and sends it to udp receiver.
 * Remote UDP receiver address and port are read from file "ipaddress.txt", which should be in the
 * same directory as this executable.
 */
int main(int argc, char* argv[]) {
	OCKAM_TRANSPORT_HANDLE		h_connection = NULL;
	OCKAM_ERR					error = 0;
	OCKAM_DEVICE_RECORD			device;
	char						buffer[128];
	unsigned int                bytes_received = 0;

	init_err_log(stdout);

	// Get server device record
	error = ockam_get_device_record( &device );
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_get_device_record");
		goto exit_block;
	}

	error = ockam_init_posix_socket_udp_server(&h_connection, &device);
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_xp_init_IP_CONNECTION");
		goto exit_block;
	}

	do {
		error = ockam_receive( h_connection, &buffer[0], sizeof( buffer ), &bytes_received );
		if ( OCKAM_ERR_NONE != error ) {
			log_error( "failed ockam_xp_receive" );
			goto exit_block;
		}

		printf( "%d Bytes, %s\n", bytes_received, buffer );
	} while ( 'q' != buffer[0] );

exit_block:
	if( NULL != h_connection ){
		ockam_uninit_transport( h_connection );
	}
	return 0;
}
