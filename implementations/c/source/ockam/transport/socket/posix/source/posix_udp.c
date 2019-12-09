#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include "posix_socket.h"
#include "error.h"
#include "errlog.h"

///////////////////////////////////////////////////////////////////////////////
//
//				Client Side
//
///////////////////////////////////////////////////////////////////////////////

/*
	Initializes one transport client connection instance
 */
OCKAM_ERR ockam_init_posix_socket_udp_client( OCKAM_CONNECTION_HANDLE* p_handle,
                                      OCKAM_DEVICE_RECORD* p_ockam_device ) {
    OCKAM_ERR		status			= OCKAM_ERR_NONE;
    TCP_CLIENT*		p_client		= NULL;

	// Allocate memory for connection data and init to 0
    p_client = (TCP_CLIENT*)malloc(sizeof(TCP_CLIENT));
    if( NULL == p_client ) {
        log_error("failed to allocate memory");
        status = OCKAM_ERR_MEM_INSUFFICIENT;
        goto exit_block;
    }
    memset(p_client, 0, sizeof(*p_client));
    p_client->type = POSIX_UDP_CLIENT;

    // Get the host IP address and port
    memcpy( &p_client->server_ockam_address, &p_ockam_device->host_address, sizeof(p_client->server_ockam_address));
    p_client->server_port = p_ockam_device->host_port;

	// Construct the server address for connection
	status = make_socket_address(
			&p_client->server_ockam_address.ip_address[0],
			p_client->server_port,
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
	*p_handle = (OCKAM_CONNECTION_HANDLE)p_client;
	return status;
 }

 /*
 	Sends a buffer to the server.
  */
 OCKAM_ERR posix_socket_udp_send(OCKAM_CONNECTION_HANDLE handle,
  	void* buffer, unsigned int length, unsigned int* p_bytes_sent
	) {

    UDP_CLIENT*			    p_client = (UDP_CLIENT*)handle;
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

	//
	printf("Sent %ld bytes out of %d, %s\n", bytes_sent, length, (char*)buffer);
	*p_bytes_sent = bytes_sent;

exit_block:
    if(-1 != p_client->socket){
    	close(p_client->socket);
    }
	return status;
}

/*
	Closes client connection and frees resources
 */
OCKAM_ERR uninit_posix_socket_udp_client( OCKAM_CONNECTION_HANDLE handle ) {
	UDP_CLIENT*			p_udp	= NULL;

	if( NULL != handle ){
		p_udp = (UDP_CLIENT*)handle;
	} else {
		goto exit_block;
	}

	free( p_udp );

exit_block:
	return OCKAM_ERR_NONE;
}

///////////////////////////////////////////////////////////////////////////////
//
//				Server Side
//
///////////////////////////////////////////////////////////////////////////////




/**
 *  ockam_init_posix_socket_udp_server
 *
 * @param p_handle This will receive the connection handle
 * @param p_ockam_device Pointer to Ockam device record
 * @return If successful, OCKAM_ERR_NONE. Otherwise see ockam_transport.h for error codes.
 */
OCKAM_ERR ockam_init_posix_socket_udp_server( OCKAM_CONNECTION_HANDLE* p_handle,
                                    OCKAM_DEVICE_RECORD* p_ockam_device ) {

	OCKAM_ERR				status			= OCKAM_ERR_NONE;
	UDP_SERVER*	            p_server		= NULL;
	char                    hostname[MAX_HOST_NAME_LENGTH];
	int                     in_status;

	// Allocate memory for connection data and init to 0
	p_server = (UDP_SERVER*)malloc(sizeof(UDP_SERVER));
	if( NULL == p_server ) {
		log_error("failed to allocate memory in ockam_init_posix_socket_udp_server");
		status = OCKAM_ERR_MEM_INSUFFICIENT;
		goto exit_block;
	}
	memset(p_server, 0, sizeof(*p_server));
	p_server->type = POSIX_UDP_SERVER;

	// Record port
	p_server->port = p_ockam_device->host_port;

	// Initialize socket
	p_server->socket = socket(AF_INET, SOCK_DGRAM, 0);
	if( -1 == p_server->socket) {
		log_error("failed to create listen socket in ockam_init_posix_socket_udp_server");
		status = OCKAM_ERR_TRANSPORT_SERVER_INIT;
		goto exit_block;
	}

	// Form the network-friendly address
	status = make_socket_address(p_ockam_device->host_address.ip_address,
	                             p_ockam_device->host_port,
	                             &p_server->socket_in_address);
	if( OCKAM_ERR_NONE != status ){
		log_error("make_socket_address failed in ockam_init_posix_socket_udp_server");
		goto exit_block;
	}

	// Bind address to socket
	if( 0 != bind(p_server->socket,
	              (struct sockaddr*)&p_server->socket_in_address,
	              sizeof(p_server->socket_in_address))
			) {
		log_error("bind failed in ockam_init_posix_socket_udp_server");
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
	*p_handle = (OCKAM_CONNECTION_HANDLE)p_server;
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
OCKAM_ERR posix_socket_udp_receive( OCKAM_CONNECTION_HANDLE handle,
                            void* p_buffer, unsigned int length, unsigned int* p_bytes_received) {
	OCKAM_ERR				status			= OCKAM_ERR_NONE;
	UDP_SERVER*	            p_server		= NULL;
	socklen_t               address_length  = 0;

	if(NULL == handle) {
		status = OCKAM_ERR_TRANSPORT_HANDLE;
		goto exit_block;
	}
	p_server = (UDP_SERVER*)handle;

	p_server->receive_transmission.p_buffer = p_buffer;
	p_server->receive_transmission.size_buffer = length;

	// Read a buffer
	address_length = sizeof( p_server->receive_transmission.sender_address );
	p_server->receive_transmission.bytes_received  = recvfrom( p_server->socket,
			p_server->receive_transmission.p_buffer,
			p_server->receive_transmission.size_buffer,
			0, NULL, 0);
//			( struct sockaddr *)&p_server->receive_transmission.sender_address,
//			&address_length );
	printf( "Received %s\n", (char*)p_server->receive_transmission.p_buffer );
	*p_bytes_received = p_server->receive_transmission.bytes_received;

exit_block:
	return status;
}

OCKAM_ERR ockam_uninit_posix_socket_udp_server( OCKAM_CONNECTION_HANDLE handle ) {
    UDP_SERVER*	        p_server	= NULL;

	if( NULL != handle ){
		p_server = (UDP_SERVER*)handle;
	} else {
		goto exit_block;
	}

	if(0 != p_server->socket ) {
		close( p_server->socket );
	}
	free( p_server );

exit_block:
	return OCKAM_ERR_NONE;
}