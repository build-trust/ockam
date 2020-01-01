#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include "posix_socket.h"
#include "error.h"
#include "errlog.h"

/**
 * ockam_init_posix_socket_udp - Initialize a posix UDP socket
 * @param p_handle
 * @param p_ockam_device
 * @return
 */
OCKAM_ERR ockam_init_posix_socket_udp_client( OCKAM_INTERNET_ADDRESS* p_address, OCKAM_TRANSPORT* p_transport )
{
    OCKAM_ERR		            status			= OCKAM_ERR_NONE;
    TRANSPORT_POSIX_UDP*		p_client		= NULL;

	// Allocate memory for connection data and init to 0
	p_client = (TRANSPORT_POSIX_UDP*)malloc(sizeof(TRANSPORT_POSIX_UDP));
    if( NULL == p_client ) {
        log_error("failed to allocate memory");
        status = OCKAM_ERR_MEM_INSUFFICIENT;
        goto exit_block;
    }
    memset(p_client, 0, sizeof(*p_client));
    p_client->type = POSIX_UDP_CLIENT;

    p_client->port = p_address->port;

	// Construct the server address for connection
	status = make_socket_address(
			&p_address->ip_address[0],
			p_address->port,
			&p_client->server_ip_address );
	if( OCKAM_ERR_NONE != status ) {
        log_error("make_socket_address failed in ockam_xp_init_tcp_client");
        status = OCKAM_ERR_INVALID_PARAM;
    }

exit_block:
	if( OCKAM_ERR_NONE != status ){
		if(NULL != p_client) {
			free( p_client );
			p_client = NULL;
		}
	}
	*p_transport = (OCKAM_TRANSPORT)p_client;
	return status;
 }

 /**
  * posix_socket_udp_send
  * @param handle
  * @param buffer
  * @param length
  * @param p_bytes_sent
  * @return
  */
 OCKAM_ERR posix_socket_udp_send(OCKAM_TRANSPORT handle,
  	void* buffer, unsigned int length, unsigned int* p_bytes_sent )
{
 	TRANSPORT_POSIX_UDP*	p_client = (TRANSPORT_POSIX_UDP*)handle;
    OCKAM_ERR				status = OCKAM_ERR_NONE;
    ssize_t					bytes_sent = 0;

     // Initialize the socket
     p_client->socket = socket(AF_INET, SOCK_DGRAM, 0);
     if( -1 == p_client->socket ) {
         log_error(("socket failed in posix_socket_udp_send"));
         status = OCKAM_ERR_TRANSPORT_INIT_SOCKET;
         goto exit_block;
     }

	 bytes_sent = sendto( p_client->socket, buffer, (size_t)length,
	 		0, (const struct sockaddr*)&p_client->server_ip_address,
            sizeof(p_client->server_ip_address) );
     if(bytes_sent < 0) {
     	log_error( "sendto() failed in posix_socket_udp_send" );
     	printf("\nerrno: %d\n", errno);
		status = OCKAM_ERR_TRANSPORT_SEND;
		goto exit_block;
	}

	*p_bytes_sent = bytes_sent;

exit_block:
    if(-1 != p_client->socket){
    	close(p_client->socket);
    }
	return status;
}

/**
 *  ockam_init_posix_socket_udp_server
 *
 * @param p_handle This will receive the connection handle
 * @param p_ockam_device Pointer to Ockam device record
 * @return If successful, OCKAM_ERR_NONE. Otherwise see ockam_transport.h for error codes.
 */
OCKAM_ERR ockam_init_posix_socket_udp_server( OCKAM_INTERNET_ADDRESS* p_address, OCKAM_TRANSPORT* p_transport )
{
	OCKAM_ERR				status			= OCKAM_ERR_NONE;
	TRANSPORT_POSIX_UDP*	p_server		= NULL;

	// Allocate memory for connection data and init to 0
	p_server = (TRANSPORT_POSIX_UDP*)malloc(sizeof(TRANSPORT_POSIX_UDP));
	if( NULL == p_server ) {
		log_error("failed to allocate memory in ockam_init_posix_socket_udp_server");
		status = OCKAM_ERR_MEM_INSUFFICIENT;
		goto exit_block;
	}
	memset(p_server, 0, sizeof(*p_server));
	p_server->type = POSIX_UDP_SERVER;

	// Record port
	p_server->port = p_address->port;

	// Initialize socket
	p_server->socket = socket(AF_INET, SOCK_DGRAM, 0);
	if( -1 == p_server->socket) {
		log_error("failed to create listen socket in ockam_init_posix_socket_udp_server");
		status = OCKAM_ERR_TRANSPORT_SERVER_INIT;
		goto exit_block;
	}

	// Form the network-friendly address
	status = make_socket_address(p_address->ip_address,
	                             p_address->port,
	                             &p_server->socket_in_address);
	if( OCKAM_ERR_NONE != status ){
		log_error("make_socket_address failed in ockam_init_posix_socket_udp_server");
		goto exit_block;
	}

	// Bind address to socket
	if ( 0 != bind( p_server->socket,
	                ( struct sockaddr * ) &p_server->socket_in_address,
	                sizeof( p_server->socket_in_address ))
			) {
		printf( "Errno: %d\n", errno);
		log_error( "bind failed in ockam_init_posix_socket_udp_server" );
		status = OCKAM_ERR_TRANSPORT_RECEIVE;
		goto exit_block;
	}
	// #revisit - this is for test feedback
	char address_buffer[80];
	const char* p_addr_buffer = NULL;
	p_addr_buffer = inet_ntop(AF_INET, &p_server->socket_in_address.sin_addr, &address_buffer[0], 80);
	printf("Receiver address %s\n", p_addr_buffer);

exit_block:
	if( OCKAM_ERR_NONE != status ){
		if( NULL != p_server ) {
			free(p_server);
			p_server = NULL;
		}
	}
	*p_transport = (OCKAM_TRANSPORT)p_server;
	return status;
}

/**
 *
 * @param handle
 * @param p_buffer
 * @param length
 * @param p_bytes_received
 * @return
 */
OCKAM_ERR posix_socket_udp_receive( OCKAM_TRANSPORT transport,
                            void* p_buffer, unsigned int length, unsigned int* p_bytes_received)
{
	OCKAM_ERR				status			= OCKAM_ERR_NONE;
	TRANSPORT_POSIX_UDP*	p_server		= NULL;
	socklen_t               address_length  = 0;

	if(NULL == transport) {
		status = OCKAM_ERR_TRANSPORT_HANDLE;
		goto exit_block;
	}
	p_server = (TRANSPORT_POSIX_UDP*)transport;

	p_server->receive_transmission.p_buffer = p_buffer;
	p_server->receive_transmission.size_buffer = length;

	// Read a buffer
	address_length = sizeof( p_server->receive_transmission.sender_address );
	p_server->receive_transmission.bytes_received  = recvfrom( p_server->socket,
			p_server->receive_transmission.p_buffer,
			p_server->receive_transmission.size_buffer,
			0, NULL, 0);
	*p_bytes_received = p_server->receive_transmission.bytes_received;

exit_block:
	return status;
}

/**
* uninit_posix_socket_udp_client - Shuts down a UDP transport instance, frees resources
* @param handle - (in) Handle to an intialized transport instance
* @return - OCKAM_NO_ERR on success
*/
OCKAM_ERR uninit_posix_socket_udp( OCKAM_TRANSPORT handle )
{
	TRANSPORT_POSIX_UDP*    p_transport 	= NULL;

	if( NULL != handle ){
		p_transport = (TRANSPORT_POSIX_UDP*)handle;
	} else {
		goto exit_block;
	}

	if(0 != p_transport->socket ) {
		close( p_transport->socket );
	}
	free( p_transport );

exit_block:
	return OCKAM_ERR_NONE;
}
