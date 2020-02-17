/**
 ********************************************************************************************************
 * @file        connection.h
 * @brief       Defines the different connection types.
 ********************************************************************************************************
 */


#ifndef TEST_TRANSPORT_CONNECTION_H
#define TEST_TRANSPORT_CONNECTION_H
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include "ockam/transport.h"

#define     DEFAULT_TCP_LISTEN_PORT         8000

typedef OCKAM_ERR (*LISTEN_CALLBACK)( OCKAM_TRANSPORT_CONNECTION connection, void* callback_context );


/**
 * This is the CONNECTION interface. Every connection type must:
 *      (a) Implement each of these functions, even if it's a no-op
 *      (b) Put a pointer to an initialized instance of this table at the beginning of its class data structure
 *      (c) Communicate with the application only through the interface defined in "transport.h". The
 *          functions in "transport.c" pass through to the function pointers in the CONNECTION_INTERFACE.
 *
 *  (See the CONNECTION definition at the end of this file)
 */
typedef struct {
    uint16_t    is_initialized;

    // listen functions
    OCKAM_ERR (*listen_blocking)( OCKAM_TRANSPORT_CONNECTION listener,
    OCKAM_LISTEN_ADDRESS* address, OCKAM_TRANSPORT_CONNECTION* connection );
    OCKAM_ERR (*listen_non_blocking)( void* address, uint16_t max_connections, LISTEN_CALLBACK cb, void* cb_context );

    // connect functions
    OCKAM_ERR (*connect_blocking)( void* address, OCKAM_TRANSPORT_CONNECTION* connection );
    OCKAM_ERR (*connect_non_blocking)( void* address, OCKAM_TRANSPORT_CONNECTION* connection );

    // receive functions
    OCKAM_ERR (*receive_blocking)( OCKAM_TRANSPORT_CONNECTION connection,
    void* buffer, uint16_t length, uint16_t* p_bytes_received);
    OCKAM_ERR (*receive_non_blocking)();

    // send functions
    OCKAM_ERR (*send_blocking)( OCKAM_TRANSPORT_CONNECTION connection, void* buffer, uint16_t length );
    OCKAM_ERR (*send_non_blocking)();

    // uninit
    OCKAM_ERR (*uninitialize)( OCKAM_TRANSPORT_CONNECTION connection );
} CONNECTION_INTERFACE;

/**
 * One TRANSMISSION instance is assigned for each read or write
 */
typedef struct {
    void*                       p_buffer;               // buffer to transmit (user-allocated)
    uint16_t                    buffer_size;            // number of bytes to transmit (write) or buffer size (read)
    uint16_t                    bytes_transmitted;      // number of bytes transmitted (so far)
    OCKAM_ERR                   completion_status;      // transmission completion status
} TRANSMISSION;



/**
 * The POSIX_SOCKET is the posix socket specific class data for a posix socket connection (TCP or UDP).
 * Note that TCP sockets are further defined by the POSIX_TCP_SOCKET type.
 */
typedef struct {
    uint16_t                    is_connected;           // connection with remote is established
    OCKAM_INTERNET_ADDRESS      local_address;          // human-friendly local address
    OCKAM_INTERNET_ADDRESS      remote_address;         // human-friendly remote address
    int                         socket;                 // posix socket identifier
    struct sockaddr_in          socket_address;         // network-friendly socket information
    TRANSMISSION                receive_transmission;
    TRANSMISSION                send_transmission;
} POSIX_SOCKET;

/**
 * For POSIX_TCP_SOCKETs, each transmission of a user's buffer is preceded by a TCP_METa_PACKET that indicates
 * the total length of the buffer. Since TCP operates on streams, this is necessary to detect when the
 * sent buffer has been completely received. Doing it this way prevents an additional memory allocation
 * and copy for each buffer sent and received.
 */
typedef struct {
    uint16_t                    this_packet_length;
    uint16_t                    next_packet_length;
} TCP_META_PACKET;

/**
 * POSIX_TCP_SOCKET has TCP-specific data.
 */
typedef struct {
    POSIX_SOCKET                posix_socket;
    LISTEN_CALLBACK             listen_callback;
    void*                       listen_context;
    TCP_META_PACKET             receive_meta;
    TCP_META_PACKET             send_meta;
} POSIX_TCP_SOCKET;

/**
 * CONNECTION is the highest-layer of abstraction for all the connections.
 */
typedef struct {
    CONNECTION_INTERFACE*       p_interface;
    union {
    POSIX_SOCKET            posix_socket;
    POSIX_TCP_SOCKET        posix_tcp_socket;
    } type;
} CONNECTION;

#endif //TEST_TRANSPORT_CONNECTION_H
