//
// Created by Robin Budd on 191226.
//

#ifndef TEST_TRANSPORT_CONNECTION_H
#define TEST_TRANSPORT_CONNECTION_H
#include "transport.h"
#include "queue.h"

#define     DEFAULT_TCP_LISTEN_PORT         8000

typedef OCKAM_ERR (*LISTEN_CALLBACK)( OCKAM_TRANSPORT_CONNECTION connection, void* callback_context );

/**
 * This is the CONNECTION interface. Every connection type must:
 *      (a) Implement each of these functions, even if it's a no-op
 *      (b) Put a pointer to an initialized instance of this table at the beginning of its class data structure
 *      (c) Communicate with the application only through this interface
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

	// read functions
	OCKAM_ERR (*read_blocking)();
	OCKAM_ERR (*read_non_blocking)();

	// write functions
	OCKAM_ERR (*write_blocking)();
	OCKAM_ERR (*write_non_blocking)();

	// uninit
	OCKAM_ERR (*uninitialize)( OCKAM_TRANSPORT_CONNECTION connection );
} CONNECTION_INTERFACE;

/**
 * The POSIX_SOCKET is the posix socket specific class data for a posix socket connection (TCP or UDP).
 * Note that TCP sockets are further
 */
typedef struct {
	uint16_t                    is_connected;           // connection with remote is established
	OCKAM_INTERNET_ADDRESS      local_address;          // human-friendly local address
	OCKAM_INTERNET_ADDRESS      remote_address;         // human-friendly remote address
	int                         socket;                 // posix socket identifier
	struct sockaddr_in          socket_address;         // network-friendly socket information
	OCKAM_QUEUE*                p_read_q;               // queue of pending read requests
	OCKAM_QUEUE*                p_write_q;              // queue of pending write requests
} POSIX_SOCKET;

/**
 * One TRANSMISSION instance is assigned for each read or write and placed in the POSIX_SOCKET's read or write queue
 */
typedef struct {
	void*                       p_buffer;               // buffer to transmit (user-allocated)
	uint16_t                    buffer_size;            // number of bytes to transmit (write) or buffer size (read)
	uint16_t                    bytes_transmitted;      // number of bytes transmitted (so far)
	OCKAM_ERR                   completion_status;      // transmission completion status
} TRANSMISSION;

/**
 * For POSIX_TCP_SOCKETs, each transmission of a user's buffer is preceded by a TCP_MET_PACKET that indicates
 * the total length of the buffer. Since TCP operates on streams, this is necessary to detect when the
 * transmission is complete.
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
	TCP_META_PACKET             read_meta;
	TCP_META_PACKET             write_meta;
} POSIX_TCP_SOCKET;

/**
 * CONNECTION is the highest-layer of abstraction for all the connections, effectively a base class.
 */
typedef struct {
	CONNECTION_INTERFACE*       p_interface;
	union {
		POSIX_SOCKET            posix_socket;
	} type;
} CONNECTION;

#endif //TEST_TRANSPORT_CONNECTION_H
