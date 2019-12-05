#include <stdio.h>
#include <string.h>

#include "transport.h"
#include "error.h"
#include "errlog.h"

#define SERV_TCP_PORT 8000
static char* g_host_ip_addr = "192.168.0.78";


OCKAM_ERR ockam_get_device_record(
        OCKAM_DEVICE_RECORD* p_ockam_device) {

    OCKAM_ERR status		= OCKAM_ERR_NONE;
    memset( p_ockam_device, 0, sizeof( *p_ockam_device));

    strcpy( &p_ockam_device->host_address.ip_address[0], g_host_ip_addr );
    p_ockam_device->host_port = SERV_TCP_PORT;

    exit_block:
    return status;
}


int main(int argc, char* argv[]) {
	OCKAM_CONNECTION_HANDLE		h_connection = NULL;
	OCKAM_ERR					error = 0;
	OCKAM_DEVICE_RECORD			ockam_device;
	char                        buffer[80];
	char*                       p_buffer = &buffer[0];
	unsigned long               buffer_size;
	unsigned int                bytes_sent = 0;

	init_err_log(stdout);

	error = ockam_get_device_record( &ockam_device);
    if( OCKAM_ERR_NONE != error ) {
        log_error("failed ockam_get_device_record");
        goto exit_block;
    }

	error = ockam_xp_init_tcp_client( &h_connection, &ockam_device );
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
		error = ockam_xp_send(h_connection, (void *) p_buffer, buffer_size, &bytes_sent);
		if (OCKAM_ERR_NONE != error) {
			log_error("ockam_xp_send failed");
			goto exit_block;
		}
	} while('q' != buffer[0]);

exit_block:
	if(NULL != h_connection) ockam_xp_uninit_client(h_connection);
	return 0;
}
