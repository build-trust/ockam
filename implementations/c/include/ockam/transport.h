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


// This section should go elsewhere as items are more broadly
// used than just in transport #revisit
//-------------------
// User-friendly IP and DNS addresses. Includes terminating NULL
#define	MAX_DNS_NAME_LENGTH		254
#define MAX_DNS_ADDRESS_LENGTH	128

/*
 * Set defaults. What needs to be parameterized? #revisit
 */

#define     MAX_HOST_NAME_LENGTH            128
#define     DEFAULT_LISTEN_PORT             8000
#define     MAX_CONNECTIONS                 50

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES                                             *
 ********************************************************************************************************
 */

// Opaque to clients, this is a pointer to a connection record and is
// cast as such in transport functions.
typedef	void*			OCKAM_CONNECTION_HANDLE;

// User-friendly internet addresses, includes terminating NULL
typedef struct {
	char					dns_name[MAX_DNS_NAME_LENGTH];			// "www.name.ext"
	char					ip_address[MAX_DNS_ADDRESS_LENGTH]; 	//"xxx.xxx.xxx.xxx"
} OCKAM_INTERNET_ADDRESS;

// Placeholder for official Ockam ID #revisit
typedef	unsigned long	OCKAM_ID;

// Placefholder for various Ockam device information #revisit
typedef struct {
	OCKAM_INTERNET_ADDRESS		host_address;
    int                         host_port;
} OCKAM_DEVICE_RECORD;

/*
 ********************************************************************************************************
 *                                       PRIVATE DATA TYPES                                             *
 ********************************************************************************************************
 */

/*
 * For reference...Socket address, posix style. Note: sockaddr_in and sockaddr can be and usually are
 * used interchangeably

struct sockaddr_in {
	__uint8_t       sin_len;
	sa_family_t     sin_family;
	in_port_t       sin_port;
	struct  in_addr sin_addr;
	char            sin_zero[8];
};

*/

// There is one transmission buffer per client send/receive. Since one client
//
typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_sent;
} TCP_TRANSMIT_SEND;

typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_received;
} TCP_TRANSMIT_RECEIVE;

typedef struct {
	int                         socket;
	struct sockaddr_in          socket_address;
	TCP_TRANSMIT_RECEIVE        receive_transmission;
	TCP_TRANSMIT_SEND           send_transmission;
} TCP_CONNECTION;

typedef struct {
	int                         socket_listen;
	int                         port_listen;
	struct sockaddr_in          socket_address_listen;
	TCP_CONNECTION              connection;
} TCP_SERVER;

typedef struct {
	OCKAM_INTERNET_ADDRESS      server_ockam_address;
	int                         socket;
	int                         server_port;
	struct sockaddr_in          server_ip_address;
	TCP_CONNECTION              connection;
} TCP_CLIENT;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/**
 * ockam_xp_init_tcp_client - Initializes a TCP client connection. If completed successfully,
 * 							ockam_xp_uninit_client must be called to free resources before exiting
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to Ockam device record of TCP host
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_xp_init_tcp_client( OCKAM_CONNECTION_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_xp_init_tcp_server - Initializes a TCP server connection. If completed successfully,
 * 							ockam_xp_uninit_server must be called to free resources before exiting.
 * @param p_handle - (out) A non-NULL value will be returned upon success
 * @param p_ockam_device - (in) Pointer to device record of this (host) device
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_xp_init_tcp_server( OCKAM_CONNECTION_HANDLE* p_handle, OCKAM_DEVICE_RECORD* p_ockam_device );

/**
 * ockam_xp_send - Sends a buffer to the host server (blocking)
 * @param handle - (in) Handle to initilized client connection
 * @param buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_sent - (out) Number of bytes successfully sent
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_xp_send(OCKAM_CONNECTION_HANDLE handle,
		void* buffer, unsigned int length, unsigned int* p_bytes_sent);

/**
 * ockam_xp_receive - Receives a buffer from a client (blocking)
 * @param handle - (in) Handle to initilized server connection
 * @param buffer - (in) Pointer to receive buffer
 * @param length - (in) Size of receive buffer
 * @param p_bytes_received  - (out) Number of bytes received
 * @return - OCKAM_SUCCESS or an error code
 */
OCKAM_ERR ockam_xp_receive( OCKAM_CONNECTION_HANDLE handle,
	void* buffer, unsigned int length, unsigned int* p_bytes_received );

/**
 * ockam_xp_uninit_client - Closes client connection and frees resources
 * @param handle - Handle of connection
 * @return  - OCKAM_SUCCESS or error
 */
OCKAM_ERR ockam_xp_uninit_client( OCKAM_CONNECTION_HANDLE handle );

/**
 * ockam_xp_uninit_server - Closes server connection and frees resources
 * @param handle - Handle of connection
 * @return  - OCKAM_SUCCESS or error
 */
OCKAM_ERR ockam_xp_uninit_server( OCKAM_CONNECTION_HANDLE handle );

#endif
