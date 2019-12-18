#ifndef POSIX_SOCKET_H
#define POSIX_SOCKET_H 1

#include <errno.h>
#include "transport.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES                                             *
 ********************************************************************************************************
 */


/*
 ********************************************************************************************************
 *                                        PRIVATE DATA TYPES                                            *
 ********************************************************************************************************
 */

/**
 * SOCKET_TYPE - Type of socket. This will likely morph into a transport type as other transports
 * (e.g. bluetooth) are implemented.
 */
typedef enum {
	POSIX_SOCKET_TYPE_ERROR                                             = 0x0000,
	POSIX_TCP_SERVER                                                    = 0x0001,
	POSIX_TCP_CLIENT                                                    = 0x0002,
	POSIX_UDP_SERVER                                                    = 0x0003,
	POSIX_UDP_CLIENT                                                    = 0x0004
} SOCKET_TYPE;

/*
 * #revisit
 * The next section (send/receive data types) will be heavily modified after a 3rd transport
 * is implemented and the code base is modified to an OOP architecture. For now, this works.
 */

/*
 * One UDP_TRANSMIT_SEND is allotted to each send request.
 */
typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_sent;
} UDP_TRANSMIT_SEND;

/*
 * One UDP_TRANSMIT_RECEIVE is allotted to each receive request.
 */
typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_received;
	struct sockaddr_in          sender_address;
} UDP_TRANSMIT_RECEIVE;

/*
 * One TCP_TRANSMIT_SEND is allotted to each send request. When bytes_sent == size_buffer,
 * the send is complete.
 */
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

/*
 * This represents an established TCP connection. At any time, a TCP connection
 * can have one active send and one active receive transmission in progress.
 */
typedef struct {
	int                         socket;
	struct sockaddr_in          socket_address;
	TCP_TRANSMIT_RECEIVE        receive_transmission;
	TCP_TRANSMIT_SEND           send_transmission;
} TCP_CONNECTION;

/*
 * The following are the top-level structures that connection handles point to.
 * The first field must ALWAYS be the type of connection.
 */

typedef struct {
	SOCKET_TYPE                 type;
	int                         socket_listen;
	in_port_t                   port_listen;
	struct sockaddr_in          socket_in_address_listen;
	TCP_CONNECTION*             p_connection;     // #revisit, there can be many connections
} TRANSPORT_POSIX_TCP_SERVER;

typedef struct {
	SOCKET_TYPE                 type;
	OCKAM_INTERNET_ADDRESS      server_ockam_address;
	int                         socket;
	struct sockaddr_in          server_ip_address;
	TCP_CONNECTION              connection;
} TRANSPORT_POSIX_TCP_CLIENT;

typedef struct {
	SOCKET_TYPE                 type;
	int                         socket;
	in_port_t                   port;
	struct sockaddr_in          socket_in_address;
	struct sockaddr_in          server_ip_address;
	UDP_TRANSMIT_RECEIVE        receive_transmission;
	UDP_TRANSMIT_SEND           send_transmission;
} TRANSPORT_POSIX_UDP;

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/**
 * make_socket_address - construct network-friendly address from user-friendly address
 * @param p_ip_address - (in) IP address in "nnn.nnn.nnn.nnn" format
 * @param port - port number, must be non-zero
 * @param p_socket_address - (out) address converted
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR make_socket_address( char* p_ip_address, in_port_t port,
		struct sockaddr_in* p_socket_address );

/**
 * posix_socket_tcp_send - Sends a buffer over a tcp connection
 * @param handle - (in) Handle to an intialized transport instance
 * @param p_buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_sent - (out) Number of bytes successfully sent
 * @return - OCKAM_NO_ERR on success
 * */
OCKAM_ERR posix_socket_tcp_send(OCKAM_TRANSPORT handle,
                                void* p_buffer, unsigned int length, unsigned int* p_bytes_sent );

/**
 * posix_socket_udp_send - Sends a buffer over a udp connection
 * @param handle - (in) Handle to an intialized transport instance
 * @param p_buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_sent - (out) Number of bytes successfully sent
 * @return - OCKAM_NO_ERR on success
 * */
OCKAM_ERR posix_socket_udp_send(OCKAM_TRANSPORT handle,
                                void* p_buffer, unsigned int length, unsigned int* p_bytes_sent );

/**
 * posix_socket_tcp_receive - Receive a buffer over a tcp connection
 * @param handle - (in) Handle to an intialized transport instance
 * @param p_buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_received  - (out) Number of bytes received
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR posix_socket_tcp_receive( OCKAM_TRANSPORT handle,
                                    void* p_buffer, unsigned int length,
                                    unsigned int* p_bytes_received );

/**
 * posix_socket_udp_receive - Receive a buffer over a udp connection
 * @param handle - (in) Handle to an intialized transport instance
 * @param p_buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_received  - (out) Number of bytes received
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR posix_socket_udp_receive( OCKAM_TRANSPORT handle,
                                    void* p_buffer, unsigned int length,
                                    unsigned int* p_bytes_received );

/**
 * uninit_posix_socket_tcp_client - Shuts down a TCP client transport instance, frees resources
 * @param handle - (in) Handle to an intialized transport instance
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR uninit_posix_socket_tcp_client( OCKAM_TRANSPORT handle );

/**
 * uninit_posix_socket_udp_client - Shuts down a UDP transport instance, frees resources
 * @param handle - (in) Handle to an intialized transport instance
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR uninit_posix_socket_udp( OCKAM_TRANSPORT handle );

/**
 * ockam_uninit_posix_tcp_server - Closes server connection and frees resources
 * @param handle - Handle of connection
 * @return  - OCKAM_SUCCESS or error
 */
OCKAM_ERR uninit_posix_socket_tcp_server( OCKAM_TRANSPORT handle );

#endif
