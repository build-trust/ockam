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

typedef enum {
	POSIX_TCP_SERVER                                                    = 0x001,
	POSIX_TCP_CLIENT                                                    = 0x002,
	POSIX_UDP_SERVER                                                    = 0x003,
	POSIX_UDP_CLIENT                                                    = 0x004
} SOCKET_TYPE;

typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_sent;
} UDP_TRANSMIT_SEND;

typedef struct {
	void*                       p_buffer;
	unsigned long               size_buffer;
	unsigned long               bytes_received;
	struct sockaddr_in          sender_address;
} UDP_TRANSMIT_RECEIVE;

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

//  The following are the top-level structures that connection handles point to

typedef struct {
	SOCKET_TYPE                 type;
	int                         socket_listen;
	int                         port_listen;
	struct sockaddr_in          socket_in_address_listen;
	TCP_CONNECTION              connection;
} TCP_SERVER;

typedef struct {
	SOCKET_TYPE                 type;
	int                         socket;
	int                         port;
	struct sockaddr_in          socket_in_address;
	UDP_TRANSMIT_RECEIVE        receive_transmission;
	UDP_TRANSMIT_SEND           send_transmission;
} UDP_SERVER;

typedef struct {
	SOCKET_TYPE                 type;
	OCKAM_INTERNET_ADDRESS      server_ockam_address;
	int                         socket;
	int                         server_port;
	struct sockaddr_in          server_ip_address;
	TCP_CONNECTION              connection;
} TCP_CLIENT;

typedef struct {
	SOCKET_TYPE                 type;
	OCKAM_INTERNET_ADDRESS      server_ockam_address;
	int                         socket;
	int                         server_port;
	struct sockaddr_in          server_ip_address;
	UDP_TRANSMIT_RECEIVE        receive_transmission;
	UDP_TRANSMIT_SEND           send_transmission;
} UDP_CLIENT;


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
 * @return
 */
OCKAM_ERR make_socket_address( char* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address );

OCKAM_ERR posix_socket_tcp_send(OCKAM_CONNECTION_HANDLE handle,
                                void* p_buffer, unsigned int length, unsigned int* p_bytes_sent );

OCKAM_ERR posix_socket_udp_send(OCKAM_CONNECTION_HANDLE handle,
                                void* p_buffer, unsigned int length, unsigned int* p_bytes_sent );

OCKAM_ERR posix_socket_tcp_receive( OCKAM_CONNECTION_HANDLE handle,
                                    void* p_buffer, unsigned int length, unsigned int* p_bytes_received );

OCKAM_ERR posix_socket_udp_receive( OCKAM_CONNECTION_HANDLE handle,
                                    void* p_buffer, unsigned int length, unsigned int* p_bytes_received );

OCKAM_ERR uninit_posix_socket_tcp_client( OCKAM_CONNECTION_HANDLE handle );

OCKAM_ERR uninit_posix_socket_udp_client( OCKAM_CONNECTION_HANDLE handle );


#endif