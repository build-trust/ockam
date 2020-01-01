//
// Created by Robin Budd on 191227.
#include <unistd.h>
#include <pthread.h>
#include "syslog.h"
#include "queue.h"
#include "error.h"
#include "transport.h"
#include "connection.h"
#include "posix_socket.h"

/**
 * Globals
 */

CONNECTION_INTERFACE  g_tcp_connection_interface = { 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL };

/**
 * Threads
 */


/**
 *  Forward Declarations
 */
OCKAM_ERR posix_tcp_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
                                     OCKAM_LISTEN_ADDRESS* p_address,
                                     OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_listen_non_blocking( void* address, uint16_t max_connections, LISTEN_CALLBACK cb, void* cb_context );
OCKAM_ERR posix_tcp_connect_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_connect_non_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_read_blocking( OCKAM_TRANSPORT_CONNECTION connection,
		void* buffer, uint16_t length, uint16_t* bytes_received);
OCKAM_ERR posix_tcp_read_non_blocking();
OCKAM_ERR posix_tcp_write_blocking();
OCKAM_ERR posix_tcp_write_non_blocking();
OCKAM_ERR posix_tcp_uninitialize( OCKAM_TRANSPORT_CONNECTION connection );


OCKAM_ERR ockam_init_posix_tcp_connection( OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR               status = OCKAM_ERR_NONE;
	CONNECTION*             p_connection = NULL;
	POSIX_SOCKET*           p_socket;

	// If first time, fill out the interface for this connection type
	if( 0 == g_tcp_connection_interface.is_initialized ) {
		// Fill out the right function pointers for this connection type
		g_tcp_connection_interface.listen_blocking = posix_tcp_listen_blocking;
		g_tcp_connection_interface.listen_non_blocking = posix_tcp_listen_non_blocking;
		g_tcp_connection_interface.connect_blocking = posix_tcp_connect_blocking;
		g_tcp_connection_interface.connect_non_blocking = posix_tcp_connect_non_blocking;
		g_tcp_connection_interface.read_blocking = posix_tcp_read_blocking;
		g_tcp_connection_interface.read_non_blocking = posix_tcp_read_non_blocking;
		g_tcp_connection_interface.write_blocking = posix_tcp_write_blocking;
		g_tcp_connection_interface.write_non_blocking = posix_tcp_write_non_blocking;
		g_tcp_connection_interface.uninitialize = posix_tcp_uninitialize;
		g_tcp_connection_interface.is_initialized = 1;
	}

	p_connection = (CONNECTION*)malloc( sizeof( CONNECTION ) );
	if( NULL == p_connection ) {
		status = OCKAM_ERR_MEM_INSUFFICIENT;
		log_error( status, "malloc failed in ockam_init_posix_tcp_transport" );
		goto exit_block;
	}
	memset( p_connection, 0, sizeof( CONNECTION ));
	p_connection->p_interface = &g_tcp_connection_interface;

	// Initialize the read/write queues
	p_socket = &p_connection->type.posix_socket;
	status = init_queue( MAX_PENDING_READS, NULL, &p_socket->p_read_q );
	if( OCKAM_ERR_NONE != status ) {
		status = OCKAM_ERR_MEM_INSUFFICIENT;
		log_error( status, "queue init failed in ockam_init_posix_tcp_transport" );
		goto exit_block;
	}
	status = init_queue( MAX_PENDING_WRITES, NULL, &p_socket->p_write_q );
	if( OCKAM_ERR_NONE != status ) {
		status = OCKAM_ERR_MEM_INSUFFICIENT;
		log_error( status, "queue init failed in ockam_init_posix_tcp_transport" );
		goto exit_block;
	}

	*connection = p_connection;

exit_block:
	return status;
}

OCKAM_ERR posix_tcp_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
		OCKAM_LISTEN_ADDRESS* p_address,
		OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_TRANSPORT_CONNECTION      new_connection = NULL;
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	CONNECTION*                     p_connection = ( CONNECTION* )listener;
	POSIX_SOCKET*                   p_socket = &p_connection->type.posix_socket;
	POSIX_SOCKET*                   p_new_socket = NULL;
	char*                           p_local_ipaddr = NULL;
	in_port_t                       port = DEFAULT_TCP_LISTEN_PORT;

	// Create the socket
	p_socket->socket = socket(AF_INET, SOCK_STREAM, 0);
	if( -1 == p_socket->socket ) {
		status = OCKAM_ERR_TRANSPORT_SERVER_INIT;
		log_error( status, "failed to create listen socket in posix_tcp_listen_blocking" );
		goto exit_block;
	}

	// Save IP address and port and construct address, if provided
	if( NULL != p_address ) {
		memcpy( &p_socket->local_address, p_address, sizeof( p_socket->local_address )  );
		p_local_ipaddr = &(p_address->internet_address.ip_address[0]);
		port = p_address->internet_address.port;
	}

	// Construct the address
	status = make_socket_address( p_local_ipaddr, port, &p_socket->socket_address );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "local IP address invalid in posix_tcp_listen_blocking ");
		goto exit_block;
	}

	// Bind the address to the socket
	if( 0 != bind(p_socket->socket,
	              (struct sockaddr*)&p_socket->socket_address,
	              sizeof(p_socket->socket_address))
			) {
		status = OCKAM_ERR_TRANSPORT_RECEIVE;
		log_error( status, "bind failed in ockam_xp_receive");
		goto exit_block;
	}

	// Initialize the new connection
	status = ockam_init_posix_tcp_connection( &new_connection );
	if( OCKAM_ERR_NONE != status ) {
		log_error( status, "failed to create new connection in posix_tcp_listen_blocking" );
		goto exit_block;
	}
	p_new_socket = &(( CONNECTION* )new_connection)->type.posix_socket;

	// Listen and wait for connection
	if(0 != listen(p_socket->socket, 1)) {   // #revisit when multiple connections implemented
		status = OCKAM_ERR_TRANSPORT_SERVER_INIT;
		log_error( status, "Listen failed" );
		goto exit_block;
	}
	p_new_socket->socket = accept( p_socket->socket, NULL, 0);
	if (-1 == p_new_socket->socket) {
		status = OCKAM_ERR_TRANSPORT_ACCEPT;
		log_error( status, "accept failed" );
		goto exit_block;
	}
	p_new_socket->is_connected = 1;

	*connection = new_connection;

exit_block:
	if( OCKAM_ERR_NONE != status ) {
		if( -1 != p_socket->socket ) close( p_socket->socket );
		if( NULL != new_connection ) ockam_uninit_connection( new_connection );
	}
	return status;
}

OCKAM_ERR posix_tcp_connect_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	CONNECTION*                     p_connection = ( CONNECTION* )connection;
	POSIX_SOCKET*                   p_socket = &p_connection->type.posix_socket;
	OCKAM_INTERNET_ADDRESS*         p_ip_address = (OCKAM_INTERNET_ADDRESS*)address;

	// Save the host IP address and port
	memcpy( &p_socket->remote_address, p_ip_address, sizeof(*p_ip_address) );

	// Construct the server address for connection
	status = make_socket_address(
			&p_socket->remote_address.ip_address[0],
			p_socket->remote_address.port,
			&p_socket->socket_address );
	if( OCKAM_ERR_NONE != status ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "make_socket_address failed in posix_tcp_connect_blocking");
	}

	// Create the socket
	p_socket->socket = socket(AF_INET, SOCK_STREAM, 0);
	if( -1 == p_socket->socket ) {
		status = OCKAM_ERR_TRANSPORT_INIT_SOCKET;
		log_error( status, "socket failed in p_socket" );
		goto exit_block;
	}


	// Try to connect
	if(connect(p_socket->socket,
	           (struct sockaddr*)&p_socket->socket_address,
	           sizeof(p_socket->socket_address)) < 0
			){
		status = OCKAM_ERR_TRANSPORT_CONNECT;
		log_error( status, "connect failed in posix_tcp_connect_blocking");
		goto exit_block;
	}
	p_socket->is_connected = 1;

exit_block:
	return status;
}

OCKAM_ERR posix_tcp_connect_non_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
	return status;
}

OCKAM_ERR posix_tcp_listen_non_blocking( void* address, uint16_t max_connections, LISTEN_CALLBACK cb, void* cb_context )
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
	return status;
}

OCKAM_ERR posix_tcp_read_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                                   void* buffer, uint16_t length, uint16_t* bytes_received)
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
	CONNECTION*                     p_connection = (CONNECTION*)connection;
	POSIX_SOCKET*                   p_socket = NULL;

	if( NULL == connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error(status, "connection must not be NULL in posix_tcp_read_blocking");
	}

	p_socket = &p_connection->type.posix_socket;

	if( 0 == p_socket->is_connected ) {
		status = OCKEM_ERR_TRANSPORT_NOT_CONNECTED;
		log_error( status, "connection not established in posix_tcp_read_blocking");
		goto exit_block;
	}



exit_block:
	return status;
}

OCKAM_ERR posix_tcp_read_non_blocking()
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
	return status;
}
OCKAM_ERR posix_tcp_write_blocking()
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
	return status;
}
OCKAM_ERR posix_tcp_write_non_blocking()
{
	OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
	return status;
}
OCKAM_ERR posix_tcp_uninitialize( OCKAM_TRANSPORT_CONNECTION connection )
{
	OCKAM_ERR                   status = OCKAM_ERR_NONE;
	CONNECTION*                 p_connection = (CONNECTION*)connection;
	POSIX_SOCKET*               p_socket = NULL;

	if( NULL == connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "connection must not be NULL in posix_tcp_uninitialize");
		goto exit_block;
	}

	p_socket = &p_connection->type.posix_socket;

	// Close sockets and free memory
	if( p_socket->socket > 0 ) close( p_socket->socket );
	free( p_connection );

exit_block:
	return status;
}

