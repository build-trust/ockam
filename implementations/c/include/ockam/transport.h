/**
 ********************************************************************************************************
 * @file        transport.h
 * @brief       Public-facing API function prototypes for Ockam's transport library
 ********************************************************************************************************
 */
#ifndef OCKAM_TRANSPORT_H
#define OCKAM_TRANSPORT_H 1

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include <time.h>
#include "error.h"
/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */


#define	MAX_DNS_NAME_LENGTH		254     // Maximum DNS name length, including terminating NULL
#define MAX_DNS_ADDRESS_LENGTH	48      // Maximum length of text DNS address in "xxx.xxx.xxx" format
#define MAX_SERVER_CONNECTIONS  128     // #revisit - should this be configurable?
#define MAX_PENDING_READS       32      // #revisit - configurable? (per socket)
#define MAX_PENDING_WRITES      32      // #revisit - configurable? (per socket)

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES                                             *
 ********************************************************************************************************
 */

typedef enum {
	CONNECTION_TYPE_UNDEFINED               = 0,
	CONNECTION_TYPE_POSIX_TCP               = 1,
	CONNECTION_TYPE_POSIX_UDP               = 2
} OCKAM_CONNECTION_TYPE;

/**
 * OCKAM_TRANSPORT_HANDLE represents a communication channel between two entities. Virtually every
 * call into the transport library takes an OCKAM_TRANSPORT_HANDLE as an argument.
 */
typedef void*           OCKAM_TRANSPORT_CONNECTION;

/**
 * OCKAM_INTERNET_ADDRESS - User-friendly internet addresses, includes terminating NULL
 */
typedef struct {
	char					dns_name[MAX_DNS_NAME_LENGTH];			// "www.name.ext"
	char					ip_address[MAX_DNS_ADDRESS_LENGTH]; 	//"xxx.xxx.xxx.xxx"
	uint16_t                port;
} OCKAM_INTERNET_ADDRESS;

/**
 * OCKAM_LISTEN_ADDRESS - Address on which to listen for an incoming connection request.
 * Address type will vary depending on connection type.
 */
typedef union {
	OCKAM_INTERNET_ADDRESS              internet_address;
} OCKAM_LISTEN_ADDRESS;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */
/**
 *                                          ockam_init_posix_tcp_connection
 * @brief   Initializes an instance of a posix tcp socket. Allocates memory structures and fills out
 *          the interface functions dispatch table. Must be paired with a call to
 *          ockam_uninit_connection upon exit.
 * @param connection
 * @return
 */
OCKAM_ERR ockam_init_posix_tcp_connection( OCKAM_TRANSPORT_CONNECTION* connection );

/**
 *                                          ockam_listen_blocking
 *
 * @brief Waits (blocking) for a remote device to connect. If successful, the new connection
 *              is returned at *p_connection.
 *
 * @param listener (in) Initialized connection instance to listen on. The listener connection
 *              instance is unique in that it is effectively an open connection waiting for
 *              an incoming connect request from any source.
 * @param address (in) Address to listen on (connection type specific)
 * @param p_connection (out) Pointer for newly established connection
 *
 * @return OCKAM_ERR_NONE if successful
 */
OCKAM_ERR ockam_listen_blocking(  OCKAM_TRANSPORT_CONNECTION listener,
                                  OCKAM_LISTEN_ADDRESS* address, OCKAM_TRANSPORT_CONNECTION* p_connection  );

/**
 *                                          ockam_connect_blocking
 *
 * @brief Connects to a remote device identified by the address parameter. If successful, the new
 *              connection is returned at *p_connection.
 *
 * @param address (in) - pointer to connection type specific address to connect to
 * @param connection (out) - Pointer for new connection
 * @return OCKAM_ERR_NONE if successful
 */
OCKAM_ERR ockam_connect_blocking( void* p_address, OCKAM_TRANSPORT_CONNECTION* p_connection );

/**
 *                                          ockam_send_blocking
 *
 * @brief Sends a buffer over an established connection.
 *
 * @param connection (in) - initialized and connected OCKAM_TRANSPORT_CONNECTION instance
 * @param p_buffer (in) - send buffer
 * @param size (in) - number of bytes to send
 * @return OCKAM_ERR_NONE if successful
 */
OCKAM_ERR ockam_send_blocking( OCKAM_TRANSPORT_CONNECTION connection, void* p_buffer, uint16_t size );

/**
 *                                          ockam_receive_blocking
 *
 * @brief Receives a buffer of data
 *
 * @param connection (in) - initialized and connected OCKAM_TRANSPORT_CONNECTION instance
 * @param p_buffer (in) - receive buffer, must be large enough for sender's data
 * @param size (in) - size of buffer
 * @param p_bytes_received (out) number of bytes received
 * @return OCKAM_ERR_NONE if successful
 */
OCKAM_ERR ockam_receive_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                               void* p_buffer, uint16_t size, uint16_t* bytes_read );

/**
 *                                          ockam_uninit_connection
 *
 * @brief Closes a connection and frees associated resources
 *
 * @param connection (in) - initialize connection instance
 *
 * @return OCKAM_ERR_NONE if successful
 */
OCKAM_ERR ockam_uninit_connection( OCKAM_TRANSPORT_CONNECTION connection );

#endif
