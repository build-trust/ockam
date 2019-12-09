#include <stdio.h>

#include "transport.h"
#include "error.h"
#include "errlog.h"

#define SERV_TCP_PORT 8000

OCKAM_ERR ockam_get_device_record(
        OCKAM_ID id,
        OCKAM_DEVICE_RECORD* p_ockam_device) {

    OCKAM_ERR status		= OCKAM_ERR_NONE;
    FILE*       address_file;
    char        listen_address[100];
    int         bytes_read = 0;

    // Read the IP address to bind to
    address_file = fopen("ipaddress.txt", "r");
    if(NULL == address_file) {
        printf("Create a file called \"ipaddress.txt\" containing the IP address to listen on, in nnn.nnn.nnn.nnn format\n");
        status = OCKAM_ERR_INVALID_PARAM;
        goto exit_block;
    }
    fscanf(address_file, "%[^\n]", &listen_address[0]);
	fclose(address_file);

    memset( p_ockam_device, 0, sizeof( *p_ockam_device));

    strcpy( &p_ockam_device->host_address.ip_address[0], &listen_address[0] );
    p_ockam_device->host_port = SERV_TCP_PORT;

    exit_block:
    return status;
}

int main(int argc, char* argv[]) {
	OCKAM_CONNECTION_HANDLE		h_connection = NULL;
	OCKAM_ERR					error = 0;
	OCKAM_DEVICE_RECORD			device;
	char						buffer[128];
	unsigned int                bytes_received = 0;

	init_err_log(stdout);

	// Get server device record
	error = ockam_get_device_record( 101, &device );
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_get_device_record");
		goto exit_block;
	}

	error = ockam_init_posix_socket_tcp_server(&h_connection, &device);
	if( OCKAM_ERR_NONE != error ) {
		log_error("failed ockam_xp_init_IP_CONNECTION");
		goto exit_block;
	}

	error = ockam_receive(h_connection, &buffer[0], sizeof(buffer), &bytes_received);
	if (OCKAM_ERR_NONE != error) {
		log_error("failed ockam_xp_receive");
		goto exit_block;
	}

	printf("%d Bytes, %s\n", bytes_received, buffer);

exit_block:
	if( NULL != h_connection ) ockam_uninit_connection( h_connection );
	return 0;
}
