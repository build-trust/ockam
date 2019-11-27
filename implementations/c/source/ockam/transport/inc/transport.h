#ifndef TRANSPORT_H
#define TRANSPORT_H 1
#include "ockam_transport.h"

/*
 * For reference...Socket address, posix style.

struct sockaddr_in {
	__uint8_t       sin_len;
	sa_family_t     sin_family;
	in_port_t       sin_port;
	struct  in_addr sin_addr;
	char            sin_zero[8];
};

*/

//typedef struct {
//    OCKAM_SERVER_HANDLE 	    handle;
//    int                         xmit_socket;
//    OCKAM_INTERNET_ADDRESS*     p_ockam_ip_local;
//    OCKAM_INTERNET_ADDRESS*     p_ockam_ip_host;
//    struct sockaddr_in			socket_address_local;
//	struct sockaddr_in			socket_address_host;
//	int                         host_port;
//} TCP_;


typedef struct {
    void*                       p_buffer;
    unsigned long               size_buffer;
    unsigned long               bytes_transmitted;
} TCP_TRANSMISSION;

typedef struct {
    int                         socket;
    struct sockaddr_in          socket_address_transmit;
	struct sockaddr_in          peer_address;
    TCP_TRANSMISSION            transmission;
} TCP_CONNECTION;

typedef struct {
    int                         socket_listen;
	int                         port_listen;
    struct sockaddr_in          socket_address_listen;
    TCP_CONNECTION              connection;
} TCP_SERVER;

typedef struct {
    int                         socket;
    int                         server_port;
    OCKAM_INTERNET_ADDRESS      server_ockam_address;
    struct sockaddr_in          server_ip_address;
    TCP_CONNECTION              connection;
} TCP_CLIENT;


OCKAM_ERROR init_internet_address(char* p_host_name, char* p_host_address);
OCKAM_ERROR init_tcp_socket( TCP_CONNECTION* p_tcp,
		OCKAM_DEVICE_RECORD* p_device );
//OCKAM_ERROR read_from_client(TCP_RECEIVE* p_receive,
//		void* p_buffer, unsigned int buffer_size, unsigned int* p_bytes_read);

#endif
