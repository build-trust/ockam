/**
 ********************************************************************************************************
 * @file        connection.h
 * @brief       Defines the different connection types.
 ********************************************************************************************************
 */
/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <unistd.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/transport.h"
#include "connection.h"
#include "posix_socket.h"

/*
 ********************************************************************************************************
 *                                             Globals                                                  *
 ********************************************************************************************************
 */

/**
 * g_tcp_connection_interface is the function table for TCP connection types. The first element in
 * all connection types must point to the connection interface (function table) for the type.
 */
CONNECTION_INTERFACE  g_tcp_connection_interface = { 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL };


/*
 ********************************************************************************************************
 *                               Forward function prototype declarations                                *
 ********************************************************************************************************
 */
OCKAM_ERR posix_tcp_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
                                     OCKAM_LISTEN_ADDRESS* p_address,
                                     OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_listen_non_blocking( void* address, uint16_t max_connections, LISTEN_CALLBACK cb, void* cb_context );
OCKAM_ERR posix_tcp_connect_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_connect_non_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection );
OCKAM_ERR posix_tcp_receive_blocking( OCKAM_TRANSPORT_CONNECTION connection,
    void* buffer, uint16_t length, uint16_t* p_bytes_received);
OCKAM_ERR posix_tcp_receive_non_blocking();
OCKAM_ERR posix_tcp_send_blocking( OCKAM_TRANSPORT_CONNECTION connection, void* buffer, uint16_t length );
OCKAM_ERR posix_tcp_send_non_blocking();
OCKAM_ERR posix_tcp_uninitialize( OCKAM_TRANSPORT_CONNECTION connection );


/*
 ********************************************************************************************************
 *                                        Global Functions                                              *
 ********************************************************************************************************
 */
OCKAM_ERR ockam_init_posix_tcp_connection( OCKAM_TRANSPORT_CONNECTION* connection )
{
    OCKAM_ERR               status = OCKAM_ERR_NONE;
    CONNECTION*             p_connection = NULL;

    // If first time, fill out the interface for this connection type
    if( 0 == g_tcp_connection_interface.is_initialized ) {
        g_tcp_connection_interface.listen_blocking = posix_tcp_listen_blocking;
        g_tcp_connection_interface.listen_non_blocking = posix_tcp_listen_non_blocking;
        g_tcp_connection_interface.connect_blocking = posix_tcp_connect_blocking;
        g_tcp_connection_interface.connect_non_blocking = posix_tcp_connect_non_blocking;
        g_tcp_connection_interface.receive_blocking = posix_tcp_receive_blocking;
        g_tcp_connection_interface.receive_non_blocking = posix_tcp_receive_non_blocking;
        g_tcp_connection_interface.send_blocking = posix_tcp_send_blocking;
        g_tcp_connection_interface.send_non_blocking = posix_tcp_send_non_blocking;
        g_tcp_connection_interface.uninitialize = posix_tcp_uninitialize;
        g_tcp_connection_interface.is_initialized = 1;
    }

    // Allocate the memory, zero it out, and set the pointer to the interface
    p_connection = (CONNECTION*)malloc( sizeof( CONNECTION ) );
    if( NULL == p_connection ) {
        status = OCKAM_ERR_MEM_INSUFFICIENT;
        log_error( status, "malloc failed in ockam_init_posix_tcp_transport" );
        goto exit_block;
    }
    memset( p_connection, 0, sizeof( CONNECTION ));
    p_connection->p_interface = &g_tcp_connection_interface;

    *connection = p_connection;

exit_block:
    if( OCKAM_ERR_NONE != status ) {
    if( NULL != p_connection ) free( p_connection );
    }
    return status;
}

/*
 ********************************************************************************************************
 *                                        Local Functions                                               *
 ********************************************************************************************************
 */
OCKAM_ERR posix_tcp_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
    OCKAM_LISTEN_ADDRESS* p_address,
    OCKAM_TRANSPORT_CONNECTION* p_new_connection )
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
        printf("errno: %d\n", errno);
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

    // Listen
    if(0 != listen(p_socket->socket, 1)) {   // #revisit when multiple connections implemented
        status = OCKAM_ERR_TRANSPORT_SERVER_INIT;
        log_error( status, "Listen failed" );
        goto exit_block;
    }

    // Wait for the connection
    p_new_socket->socket = accept( p_socket->socket, NULL, 0);
    if (-1 == p_new_socket->socket) {
        status = OCKAM_ERR_TRANSPORT_ACCEPT;
        log_error( status, "accept failed" );
        goto exit_block;
    }
    p_new_socket->is_connected = 1;

    // It all worked. Copy the new connection to the caller's variable.
    *p_new_connection = new_connection;

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

OCKAM_ERR posix_tcp_receive_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                                   void* buffer, uint16_t length, uint16_t* p_bytes_received)
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    CONNECTION*                     p_connection = (CONNECTION*)connection;
    POSIX_TCP_SOCKET*               p_tcp = NULL;
    TRANSMISSION*                   p_transmission;
    ssize_t                         bytes_read = 0;

    if( NULL == connection ) {
        status = OCKAM_ERR_INVALID_PARAM;
        log_error(status, "connection must not be NULL in posix_tcp_receive_blocking");
    }

    p_tcp = &p_connection->type.posix_tcp_socket;
    p_transmission = &p_tcp->posix_socket.receive_transmission;

    if( 1 != p_tcp->posix_socket.is_connected ) {
        status = OCKAM_ERR_TRANSPORT_NOT_CONNECTED;
        log_error( status, "tcp socket must be connected for read operation" );
        goto exit_block;
    }

    // Read the metadata buffer
    bytes_read = recv( p_tcp->posix_socket.socket, &p_tcp->receive_meta, sizeof( p_tcp->receive_meta), 0 );
    if( sizeof( p_tcp->receive_meta ) != bytes_read ) {
        status = OCKAM_ERR_TRANSPORT_RECEIVE;
        log_error( status, "failed to read metadata buffer" );
        goto exit_block;
    }

    // Fix endian-ness
    p_tcp->receive_meta.next_packet_length = ntohs( p_tcp->receive_meta.next_packet_length );
    p_tcp->receive_meta.this_packet_length = ntohs( p_tcp->receive_meta.this_packet_length );

    // Sanity check that what we got was a metadata packet
    if( p_tcp->receive_meta.this_packet_length != bytes_read ) {
        status = OCKAM_ERR_TRANSPORT_RECEIVE;
        log_error( status, "expected metadata packet in posix_tcp_receive_blocking");
        goto exit_block;
    }

    // Verify the receive buffer is big enough
    if( length < p_tcp->receive_meta.next_packet_length ) {
        status = OCKAM_ERR_TRANSPORT_BUFFER_TOO_SMALL;
        log_error( status, "supplied receive buffer too small");
        goto exit_block;
    }

    // Read the actual data
    p_transmission->p_buffer = buffer;
    p_transmission->buffer_size = p_tcp->receive_meta.next_packet_length;
    bytes_read = recv(p_tcp->posix_socket.socket,
        p_transmission->p_buffer,
        p_transmission->buffer_size, 0);
    if( -1 == bytes_read ) {
        status = OCKAM_ERR_TRANSPORT_RECEIVE;
        log_error( status, "recv failed in posix_tcp_receive_blocking\n" );
    }
    p_transmission->bytes_transmitted = bytes_read;
    *p_bytes_received = bytes_read;

exit_block:
    return status;
}

OCKAM_ERR posix_tcp_receive_non_blocking()
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
exit_block:
    return status;
}
OCKAM_ERR posix_tcp_send_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                                    void* buffer, uint16_t length )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    CONNECTION*                     p_connection = (CONNECTION*)connection;
    POSIX_TCP_SOCKET*               p_tcp = NULL;
    TRANSMISSION*                   p_transmission;
    ssize_t                         bytes_sent = 0;

    if( NULL == connection ) {
        status = OCKAM_ERR_INVALID_PARAM;
        log_error(status, "connection must not be NULL in posix_tcp_send_blocking");
    }

    p_tcp = &p_connection->type.posix_tcp_socket;
    p_transmission = &p_tcp->posix_socket.receive_transmission;

    if( 1 != p_tcp->posix_socket.is_connected ) {
        status = OCKAM_ERR_TRANSPORT_NOT_CONNECTED;
        log_error( status, "tcp socket must be connected for write operation" );
        goto exit_block;
    }

    // send the meta
    p_tcp->send_meta.this_packet_length = htons( (uint16_t)sizeof( p_tcp->send_meta ));
    p_tcp->send_meta.next_packet_length = htons( length );

    bytes_sent = send( p_tcp->posix_socket.socket, &p_tcp->send_meta, sizeof( p_tcp->send_meta ), 0 );
    if( bytes_sent != sizeof( p_tcp->send_meta ) ) {
        status = OCKAM_ERR_TRANSPORT_SEND;
        log_error( status, "error sending buffer in posix_tcp_send_blocking");
        goto exit_block;
    }

    p_transmission->p_buffer = buffer;
    p_transmission->buffer_size = length;
    bytes_sent = send( p_tcp->posix_socket.socket, p_transmission->p_buffer, p_transmission->buffer_size, 0 );
    if( bytes_sent !=  p_transmission->buffer_size ) {
        status = OCKAM_ERR_TRANSPORT_SEND;
        log_error( status, "error sending buffer in posix_tcp_send_blocking");
        goto exit_block;
    }

exit_block:
    return status;
}

OCKAM_ERR posix_tcp_send_non_blocking()
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

    // Close socket and free memory
    if( p_socket->socket > 0 ) close( p_socket->socket );

    free( p_connection );

exit_block:
    return status;
}
