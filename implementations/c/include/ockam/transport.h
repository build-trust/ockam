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


/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES                                             *
 ********************************************************************************************************
 */
/*
 * Placeholder for official Ockam ID #revisit
 */
typedef	unsigned long	OCKAM_ID;

/**
 * OCKAM_TRANSPORT_HANDLE represents a communication channel between two entities. Virtually every
 * calls into the transport library takes an OCKAM_TRANSPORT_HANDLE as an argument.
 */
typedef	void*			OCKAM_TRANSPORT_HANDLE;

/**
 * OCKAM_INTERNET_ADDRESS - User-friendly internet addresses, includes terminating NULL
 */
typedef struct {
	char					dns_name[MAX_DNS_NAME_LENGTH];			// "www.name.ext"
	char					ip_address[MAX_DNS_ADDRESS_LENGTH]; 	//"xxx.xxx.xxx.xxx"
} OCKAM_INTERNET_ADDRESS;

/*
 * Placefholder for various Ockam device information #revisit
 */
typedef struct {
	OCKAM_INTERNET_ADDRESS		host_address;
	in_port_t                   host_port;
} OCKAM_DEVICE_RECORD;


/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/**
 * ockam_init_posix_socket_tcp_client - Initializes a TCP client connection. If completed successfully,
 * 							ockam_uninit_transport must be called to free resources before exiting
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to Ockam device record of TCP host
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_init_posix_socket_tcp_client( OCKAM_TRANSPORT_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_init_posix_socket_tcp_server - Initializes a TCP server connection. If completed successfully,
 * 							ockam_uninit_transport must be called to free resources before exiting.
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to device record of this (host) device
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_init_posix_socket_tcp_server( OCKAM_TRANSPORT_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_init_posix_socket_udp_server - Initializes a UDP  connection. If completed successfully,
 * 							ockam_uninit_transport must be called to free resources before exiting.
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to device record of this (host) device
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_init_posix_socket_udp_server( OCKAM_TRANSPORT_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_init_posix_socket_udp - Initializes a UDP  connection. If completed successfully,
 * 							ockam_uninit_transport must be called to free resources before exiting.
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to device record of this (host) device
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_init_posix_socket_udp_client( OCKAM_TRANSPORT_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_send - Sends a buffer to the host server (blocking)
 * @param handle - (in) Handle to initilized client connection
 * @param buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_sent - (out) Number of bytes successfully sent
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_send(OCKAM_TRANSPORT_HANDLE handle,
		void* buffer, unsigned int length, unsigned int* p_bytes_sent);

/**
 * ockam_receive - Receives a buffer from a client (blocking)
 * @param handle - (in) Handle to initilized server connection
 * @param buffer - (in) Pointer to receive buffer
 * @param length - (in) Size of receive buffer
 * @param p_bytes_received  - (out) Number of bytes received
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_receive( OCKAM_TRANSPORT_HANDLE handle,
	void* buffer, unsigned int length, unsigned int* p_bytes_received );

/**
 * ockam_uninit_transport - Closes connection and frees resources
 * @param handle - Handle of initialized transport instance
 * @return  - OCKAM_SUCCESS or error
 */
OCKAM_ERR ockam_uninit_transport( OCKAM_TRANSPORT_HANDLE handle );

#endif
